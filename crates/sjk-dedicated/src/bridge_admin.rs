//! The running mod's admin system (`sjk_server_mod::admin`), JA+'s on a `g_mod japlus`
//! server: every player's rank, the rights each rank's mask gives, the logins, the
//! log of admin commands, the anti-abuse counter, the addresses that may not log in,
//! and the server console's own admin commands. The base game has no admin system, so
//! none of this runs there.
//!
//! The behaviour is VisionJK's port of the JA+ 2.4 B7 module
//! (`codemp/game/japlus/g_japlus_admin.c`), the code SJK's. Whatever names or texts the
//! mod has are read from its [`AdminSystem`], never written here.

use super::{NativeGame, Told};
use sjk_game_jka::ip_filter::IpFilter;
use sjk_network::LegacyTokens;
use sjk_server_mod::admin::{
    self as rules, Abuse, AbuseCounter, AbuseRules, AdminCommand, AdminEntry, AdminSystem, Named,
    Rank,
};

/// Wrong logins before a player has to wait (not JA+'s: VisionJK
/// `g_japlus_admin.c:385-388`), the first wait, which doubles with each wrong try after
/// it, and the longest.
const LOGIN_TRIES: u32 = 3;
const LOGIN_WAIT_MSEC: i32 = 30_000;
const LOGIN_WAIT_MAX_MSEC: i32 = 1_800_000;

/// What the server keeps of one player for the admin system.
#[derive(Clone, Debug, Default)]
pub(crate) struct AdminState {
    /// `sess.jpAdminLevel`: its rank, kept over a map change.
    pub(crate) rank: Option<Rank>,
    /// The rank came from its key: a new `hello` after a map change is answered without
    /// a new login.
    pub(crate) by_key: bool,
    abuse: AbuseCounter,
    /// Wrong logins, and until when it may not try again.
    login_fails: u32,
    login_until: i32,
}

impl AdminState {
    /// What a map change keeps (`sess`): the rank, nothing of the level's.
    pub(crate) fn kept(&self) -> Self {
        Self {
            rank: self.rank,
            by_key: self.by_key,
            ..Self::default()
        }
    }
}

/// What the server keeps for the admin system beside its players.
#[derive(Debug, Default)]
pub(crate) struct AdminServer {
    /// The admin-ban list (`adminBanList`, the setting `jp_banAdminIPs`).
    filter: IpFilter,
    /// What the server console's admin commands answered, printed when the line is done.
    console_out: Vec<u8>,
}

/// Who runs an admin command: a player, or the server console (and the remote one),
/// which has every right (`rcon` in VisionJK's functions).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Caller {
    Console,
    Player(usize),
}

impl Caller {
    fn player(self) -> Option<usize> {
        match self {
            Caller::Console => None,
            Caller::Player(client) => Some(client),
        }
    }
}

/// A `print "..."` command.
fn print_command(text: &[u8]) -> Vec<u8> {
    [&b"print \""[..], text, b"\""].concat()
}

/// `text` (a fixed text of the mod, with at most one `%s` and one `%i` or two `%i`)
/// with its numbers and its word filled in, in their order.
fn fill(text: &str, words: &[&[u8]], numbers: &[i32]) -> Vec<u8> {
    let (mut words, mut numbers) = (words.iter(), numbers.iter());
    let mut out = Vec::with_capacity(text.len() + 16);
    let mut bytes = text.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        match (byte, bytes.peek()) {
            (b'%', Some(b's')) => {
                bytes.next();
                out.extend_from_slice(words.next().copied().unwrap_or_default());
            }
            (b'%', Some(b'i')) => {
                bytes.next();
                out.extend_from_slice(numbers.next().copied().unwrap_or(0).to_string().as_bytes());
            }
            _ => out.push(byte),
        }
    }
    out
}

impl NativeGame {
    /// The running mod's admin system, if it has one.
    pub(super) fn admin_system(&self) -> Option<&'static AdminSystem> {
        self.server_mod.and_then(|server_mod| server_mod.admin())
    }

    /// A player's rank, `None` for none.
    pub(super) fn admin_rank(&self, client: usize) -> Option<Rank> {
        self.peer(client).and_then(|peer| peer.admin.rank)
    }

    /// What `caller` is told: a player by `print`, the console on its own output.
    pub(super) fn admin_reply(&mut self, caller: Caller, text: &[u8]) {
        match caller {
            Caller::Console => self.admin.console_out.extend_from_slice(text),
            Caller::Player(client) => self.told.push(Told::One(client, print_command(text))),
        }
    }

    /// A text of the admin system to one player.
    fn admin_tell(&mut self, client: usize, text: &[u8]) {
        self.told.push(Told::One(client, print_command(text)));
    }

    /// `JP_ValidClient`: the player at `client` if it is in the game (`CON_CONNECTED`).
    pub(super) fn admin_valid(&self, client: i32) -> Option<usize> {
        let client = usize::try_from(client).ok()?;
        (client < self.players.places() && self.peer(client).is_some_and(|peer| peer.begun))
            .then_some(client)
    }

    /// `G_getClientNumberForString`: -1 for everybody, a client number, or -2 for
    /// nobody (a name no player in the game holds).
    pub(super) fn admin_client_for(&self, word: &[u8]) -> i32 {
        match rules::named(word) {
            Named::Everybody => -1,
            Named::Number(number) => number,
            Named::Name => (0..self.players.places())
                .find(|&client| {
                    self.admin_valid(client as i32).is_some()
                        && self
                            .peer(client)
                            .is_some_and(|peer| rules::name_holds(&peer.name, word))
                })
                .map_or(-2, |client| client as i32),
        }
    }

    /// The allowed mask of `rank` (none for rcon, which needs none).
    fn admin_mask(&self, system: &AdminSystem, rank: Option<Rank>) -> i32 {
        rank.and_then(Rank::named_index).map_or(0, |index| {
            self.cvars
                .integer(system.settings.allowed[index].as_bytes())
        })
    }

    /// `G_ClientAdminCmdAllowed`: whether `caller` may use `entry`, the refusal told when
    /// `notify`. The console may use everything.
    pub(super) fn admin_allowed(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        notify: bool,
    ) -> bool {
        let (Some(system), Some(client)) = (self.admin_system(), caller.player()) else {
            return true;
        };
        let rank = self.admin_rank(client);
        let allowed = rules::allows(rank, self.admin_mask(system, rank), entry);
        if notify && !allowed {
            self.admin_tell(client, system.texts.not_allowed.as_bytes());
        }
        allowed
    }

    /// `G_ClientCheckAdminAbuse`: one more sensible command by `client`; `true` when it is
    /// refused (told when `notify`), and the admin-ban when the abuses are used up.
    pub(super) fn admin_abused(&mut self, client: usize, notify: bool) -> bool {
        let Some(system) = self.admin_system() else {
            return false;
        };
        let settings = system.settings;
        let abuse_rules = AbuseRules {
            seconds: self.cvars.integer(settings.abuse_timer.as_bytes()),
            commands: self.cvars.integer(settings.abuse_commands.as_bytes()),
            bans_before: self.cvars.integer(settings.abuse_bans.as_bytes()),
        };
        let now = self.last_frame_time;
        let Some(peer) = self.peer_mut(client) else {
            return true;
        };
        if !peer.begun {
            return true;
        }
        let rank = peer.admin.rank;
        let outcome = peer.admin.abuse.count(abuse_rules, rank, now);
        let (seconds_left, bans_left) = (
            peer.admin.abuse.seconds_left(now),
            peer.admin.abuse.bans_left,
        );
        let (name, address) = (peer.name.clone(), peer.address.clone());
        let countdown = match outcome {
            Abuse::Fine => return false,
            Abuse::Banned => {
                // A player whose address is admin-banned already only loses the try.
                if !address.is_empty() && !self.admin_banned(&address) {
                    if let Some(peer) = self.peer_mut(client) {
                        peer.admin.rank = None;
                        peer.admin.by_key = false;
                    }
                    self.admin_ban_address(&address);
                    self.log(&format!(
                        "ADMIN ABUSE DETECTED ==> admBan on Admin : {} \n",
                        String::from_utf8_lossy(&name)
                    ));
                    self.admin_tell(client, system.texts.admin_banned.as_bytes());
                }
                return true;
            }
            Abuse::Refused { first, countdown } => {
                if first {
                    self.log(&format!(
                        "ADMIN ABUSE DETECTED on Admin: {} \n",
                        String::from_utf8_lossy(&name)
                    ));
                }
                countdown
            }
        };
        if notify {
            let commands = abuse_rules.commands;
            self.admin_tell(
                client,
                &fill(system.texts.abuse_wait, &[], &[seconds_left, commands]),
            );
            if countdown {
                self.admin_tell(
                    client,
                    &fill(system.texts.abuse_countdown, &[], &[bans_left]),
                );
            }
        }
        true
    }

    /// `G_ClientAdminCmdAllowedOn`: whether `caller` may use a command on `target` (told
    /// when `notify`), with the abuse counter counted when `check_abuse` and it is not on
    /// itself. The console may use it on anybody.
    pub(super) fn admin_allowed_on(
        &mut self,
        caller: Caller,
        target: usize,
        notify: bool,
        check_abuse: bool,
    ) -> bool {
        let (Some(system), Some(client)) = (self.admin_system(), caller.player()) else {
            return true;
        };
        let connected =
            |game: &Self, client: usize| game.peer(client).is_some_and(|peer| peer.begun);
        if !connected(self, client) || !connected(self, target) {
            return false;
        }
        let oneself = client == target;
        let setting = self.cvars.integer(system.settings.usable_on.as_bytes());
        let allowed = rules::usable_on(
            self.admin_rank(client),
            self.admin_rank(target),
            oneself,
            setting,
        );
        if notify && !allowed {
            self.admin_tell(client, system.texts.not_allowed_on.as_bytes());
        }
        allowed && (oneself || !check_abuse || !self.admin_abused(client, true))
    }

    /// `JP_LogCmd`: an admin command in the log, unless the console or a player of the
    /// rcon rank ran it, as JA+ does.
    pub(super) fn admin_log_command(&mut self, caller: Caller, command: &str, extra: &[u8]) {
        let Some(client) = caller.player() else {
            return;
        };
        if self.admin_rank(client) == Some(Rank::Rcon) {
            return;
        }
        let name = self
            .peer(client)
            .map(|peer| peer.name.clone())
            .unwrap_or_default();
        let space = if extra.is_empty() { "" } else { " " };
        self.log(&format!(
            "ADMIN CMD EXECUTED : {command}{space}{} executed by {}\n",
            String::from_utf8_lossy(extra),
            String::from_utf8_lossy(&name)
        ));
    }

    /// `G_AdminFilterPacket`: whether `address` is admin-banned.
    pub(super) fn admin_banned(&self, address: &[u8]) -> bool {
        self.admin.filter.refuses(address, true)
    }

    /// `G_AdminAddIP` of `address` (its port left out), into the setting too.
    fn admin_ban_address(&mut self, address: &[u8]) {
        let host = address
            .split(|byte| *byte == b':')
            .next()
            .unwrap_or_default();
        let list = self.admin.filter.add(host, &mut |text| {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), text);
        });
        self.admin_store_bans(&list);
    }

    /// The admin-ban list written back into its setting.
    fn admin_store_bans(&mut self, list: &[u8]) {
        if let Some(system) = self.admin_system() {
            self.set_cvar(system.settings.banned_ips, &String::from_utf8_lossy(list));
        }
    }

    /// `G_ProcessAdminIPBans`, as every level starts: the setting read into a list
    /// started afresh (the game module is loaded again for a level).
    pub(super) fn process_admin_bans(&mut self) {
        self.admin.filter = IpFilter::default();
        let Some(system) = self.admin_system() else {
            return;
        };
        let list = self
            .cvars
            .string(system.settings.banned_ips.as_bytes())
            .to_vec();
        self.admin.filter.process(&list, &mut |text| {
            let _ = std::io::Write::write_all(&mut std::io::stdout(), text);
        });
    }

    /// `G_AdminLoginBlocked`: whether `client` may not try a login now, admin-banned or
    /// waiting after wrong tries (told when `notify`).
    pub(super) fn admin_login_blocked(&mut self, client: usize, notify: bool) -> bool {
        let Some(system) = self.admin_system() else {
            return true;
        };
        let now = self.last_frame_time;
        let Some(peer) = self.peer(client) else {
            return true;
        };
        let (address, until) = (peer.address.clone(), peer.admin.login_until);
        if self.admin_banned(&address) {
            if let Some(peer) = self.peer_mut(client) {
                peer.admin.rank = None;
                peer.admin.by_key = false;
            }
            if notify {
                self.admin_tell(client, system.texts.admin_banned_try.as_bytes());
            }
            return true;
        }
        if until > now {
            if notify {
                self.admin_tell(client, system.texts.too_many_tries.as_bytes());
            }
            return true;
        }
        false
    }

    /// `G_AdminLoginFailed`: a wrong try, `what` for the console's security line.
    pub(super) fn admin_login_failed(&mut self, client: usize, what: &str) {
        let now = self.last_frame_time;
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        peer.admin.login_fails += 1;
        if peer.admin.login_fails >= LOGIN_TRIES {
            let steps = peer.admin.login_fails - LOGIN_TRIES;
            let wait = LOGIN_WAIT_MSEC
                .checked_shl(steps)
                .filter(|wait| (1..=LOGIN_WAIT_MAX_MSEC).contains(wait) && steps < 16)
                .unwrap_or(LOGIN_WAIT_MAX_MSEC);
            peer.admin.login_until = now + wait;
        }
        println!(
            "amlogin: {what} from {} [{}]",
            String::from_utf8_lossy(&peer.name),
            String::from_utf8_lossy(&peer.address)
        );
    }

    /// `G_AdminLoginDone`: `client` is an admin of `rank` from now on, the rank's message
    /// told to everyone when `notify`; `how` names the way in for the console.
    pub(super) fn admin_login_done(&mut self, client: usize, rank: Rank, notify: bool, how: &str) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let message = rank.named_index().map(|index| {
            self.cvars
                .string(system.settings.login_message[index].as_bytes())
                .to_vec()
        });
        let Some(peer) = self.peer_mut(client) else {
            return;
        };
        peer.admin.rank = Some(rank);
        peer.admin.login_fails = 0;
        peer.admin.login_until = 0;
        let (name, address) = (peer.name.clone(), peer.address.clone());
        if notify {
            match &message {
                Some(message) => {
                    let text = rules::new_lines(&rules::format_name(message, &name));
                    self.told.push(Told::Everyone(print_command(
                        &[&b"^7"[..], &text, b"\n"].concat(),
                    )));
                }
                // JA+ has no line for this rank: its module had no login for it.
                None => self.admin_tell(client, system.texts.rcon_login.as_bytes()),
            }
        }
        let title = match rank {
            Rank::Council => "^3admin council",
            Rank::Knight => "^5admin knight",
            Rank::Instructor => "^5admin instructor",
            Rank::Rcon => "^1admin with every right",
        };
        let name = String::from_utf8_lossy(&name).into_owned();
        self.log(&format!("{name} is logged as an {title}.\n"));
        println!(
            "amlogin: {name} [{}] is admin of rank {} by {how}",
            String::from_utf8_lossy(&address),
            rank.level()
        );
    }

    /// `ClientCommand` for one of the admin system's commands, `text` the whole line as
    /// the player sent it.
    pub(super) fn admin_player_command(
        &mut self,
        client: usize,
        text: &[u8],
        command: AdminCommand,
        server_time: i32,
    ) {
        let words: Vec<Vec<u8>> = LegacyTokens::new(text).map(<[u8]>::to_vec).collect();
        let (Some(system), Some(name)) = (self.admin_system(), words.first()) else {
            return;
        };
        let Some(entry) = system.entry(name, false) else {
            return;
        };
        // Only the chat of the admins works during the intermission, as in JA+
        // (VisionJK `g_japlus_admin.c:1196-1218`); SJK's key login does too.
        if !matches!(command, AdminCommand::Say | AdminCommand::KeyLogin)
            && self.refused_at_intermission(client, name)
        {
            return;
        }
        let words: Vec<&[u8]> = words.iter().map(Vec::as_slice).collect();
        self.run_admin(Caller::Player(client), entry, command, &words, server_time);
    }

    /// A server console line that is one of the admin system's commands; `false` for any
    /// other line.
    pub(super) fn admin_console_command(
        &mut self,
        words: &[&[u8]],
        server_time: i32,
        print: &mut dyn FnMut(&[u8]),
    ) -> bool {
        let (Some(system), Some(name)) = (self.admin_system(), words.first()) else {
            return false;
        };
        let Some(entry) = system.entry(name, true) else {
            return false;
        };
        let Some(command) = entry.command else {
            return false;
        };
        self.run_admin(Caller::Console, entry, command, words, server_time);
        let out = std::mem::take(&mut self.admin.console_out);
        if !out.is_empty() {
            print(&out);
        }
        true
    }

    /// One admin command, by `caller`, as `words` typed it (its name first).
    fn run_admin(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        command: AdminCommand,
        words: &[&[u8]],
        server_time: i32,
    ) {
        let _ = server_time;
        match (command, caller) {
            (AdminCommand::Login, Caller::Player(client)) => self.amlogin(client, words),
            (AdminCommand::Logout, Caller::Player(client)) => self.amlogout(client, entry),
            (AdminCommand::Info, Caller::Player(client)) => self.aminfo(client),
            (AdminCommand::Kick, _) => self.amkick(caller, entry, words, false),
            (AdminCommand::Ban, _) => self.amkick(caller, entry, words, true),
            (AdminCommand::Grant, Caller::Console) => self.adm_grant(words),
            (AdminCommand::Toggle, Caller::Console) => self.adm_toggle(words),
            (AdminCommand::AdminBan, Caller::Console) => self.adm_ban(words),
            (AdminCommand::AddIp, Caller::Console) => self.adm_add_ip(words),
            (AdminCommand::RemoveIp, Caller::Console) => self.adm_remove_ip(words),
            (AdminCommand::ListIp, Caller::Console) => {
                let list = self.admin_system().map(|system| system.settings.banned_ips);
                if let Some(name) = list {
                    let value =
                        String::from_utf8_lossy(self.cvars.string(name.as_bytes())).into_owned();
                    self.admin_reply(caller, format!("\"{name}\" is:\"{value}^7\"\n").as_bytes());
                }
            }
            (_, Caller::Player(client)) => {
                // Not carried out yet: the game's own answer.
                let name = words.first().copied().unwrap_or_default();
                self.told.push(Told::One(
                    client,
                    sjk_game_jka::client_commands::unknown(name),
                ));
            }
            (_, Caller::Console) => {}
        }
    }

    /// `JP_AdminCallBegin`: the player a command names in its first word (-1 for all,
    /// -2 for nobody), once the caller's rank allows the command (told if not). `None`
    /// when the command goes no further.
    pub(super) fn admin_call_target(
        &mut self,
        caller: Caller,
        entry: &AdminEntry,
        words: &[&[u8]],
    ) -> Option<i32> {
        let target = self.admin_client_for(words.get(1)?);
        self.admin_allowed(caller, entry, true).then_some(target)
    }

    /// `amkick <player>` (`Cmd_amkick_f`) and `amban <player>` (`Cmd_amban_f`): the
    /// player kicked, after its address is added to the game's IP filter for a ban.
    fn amkick(&mut self, caller: Caller, entry: &AdminEntry, words: &[&[u8]], ban: bool) {
        let Some(target) = self.admin_call_target(caller, entry, words) else {
            return;
        };
        let Some(target) = self.admin_valid(target) else {
            return;
        };
        if !self.admin_allowed_on(caller, target, true, true) {
            return;
        }
        if ban && let Some(peer) = self.peer(target) {
            // `G_CleanIP` and `G_FilterPacket`: the host alone, unless it is banned already.
            let host = peer
                .address
                .split(|byte| *byte == b':')
                .next()
                .unwrap_or_default()
                .to_vec();
            if !host.is_empty() && !peer.bot && !self.filtered_out(&host) {
                let words: [&[u8]; 2] = [b"addip", &host];
                self.game_console_command(&words, self.last_frame_time, &mut |text| {
                    let _ = std::io::Write::write_all(&mut std::io::stdout(), text);
                });
            }
        }
        self.admin_log_command(caller, entry.name, b"");
        // `clientkick <n>`, as the reference's console runs it next.
        self.told.push(Told::Drop {
            client: target,
            reason: b"@@@WAS_KICKED".to_vec(),
        });
    }

    /// `amlogin <password or code>` (`Cmd_amlogin_f`, `G_amlogin`): a one-time code, or
    /// one of the rank passwords where the server allows them.
    fn amlogin(&mut self, client: usize, words: &[&[u8]]) {
        let Some(&typed) = words.get(1) else {
            return;
        };
        self.admin_login_typed(client, typed, true, false);
    }

    /// `G_amlogin`: what a player typed, or what its plugin sent by itself
    /// (`from_userinfo`: `cp_login`), which only counts where passwords do.
    pub(super) fn admin_login_typed(
        &mut self,
        client: usize,
        typed: &[u8],
        notify: bool,
        from_userinfo: bool,
    ) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let settings = system.settings;
        if !from_userinfo && typed.eq_ignore_ascii_case(settings.unset_password.as_bytes()) {
            self.admin_tell(client, system.texts.unset_password_typed.as_bytes());
            return;
        }
        let passwords = self.cvars.integer(settings.passwords_allowed.as_bytes()) != 0;
        if from_userinfo && !passwords {
            return;
        }
        if self.admin_login_blocked(client, notify) {
            return;
        }
        let code = (!from_userinfo)
            .then(|| self.admin_take_code(typed))
            .flatten();
        if let Some(rank) = code {
            self.admin_login_done(client, rank, notify, "one-time code");
            return;
        }
        let lost = |game: &mut Self| {
            if let Some(peer) = game.peer_mut(client) {
                peer.admin.rank = None;
                peer.admin.by_key = false;
            }
        };
        if !passwords {
            lost(self);
            self.admin_login_failed(client, "wrong one-time code");
            if notify {
                self.admin_tell(client, system.texts.no_such_code.as_bytes());
            }
            return;
        }
        let rank = Rank::NAMED
            .into_iter()
            .zip(settings.password)
            .find_map(|(rank, name)| {
                let password = self.cvars.string(name.as_bytes());
                (!password.is_empty()
                    && password != settings.unset_password.as_bytes()
                    && password == typed)
                    .then_some(rank)
            });
        match rank {
            Some(rank) => {
                if let Some(peer) = self.peer_mut(client) {
                    peer.admin.by_key = false;
                }
                self.admin_login_done(client, rank, notify, "password");
            }
            None => {
                lost(self);
                self.admin_login_failed(client, "wrong password");
                if notify {
                    self.admin_tell(client, system.texts.bad_password.as_bytes());
                }
            }
        }
    }

    /// `JP_ClientBegin`'s login (VisionJK `g_japlus_misc.c:749-757`): a player with no
    /// rank whose JA+ plugin sends a password by itself (`cp_login`), tried where the
    /// passwords count.
    pub(super) fn admin_userinfo_login(&mut self, client: usize) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let Some(peer) = self.peer(client) else {
            return;
        };
        if peer.admin.rank.is_some() {
            return;
        }
        let Some(password) = sjk_protocol::info_value(&peer.userinfo, b"cp_login")
            .filter(|value| {
                !value.is_empty()
                    && !value.eq_ignore_ascii_case(system.settings.unset_password.as_bytes())
            })
            .map(<[u8]>::to_vec)
        else {
            return;
        };
        self.admin_login_typed(client, &password, true, true);
    }

    /// A one-time code's rank, the code used up; none before the key login exists.
    fn admin_take_code(&mut self, _typed: &[u8]) -> Option<Rank> {
        None
    }

    /// `amlogout` (`Cmd_amlogout_f`).
    fn amlogout(&mut self, client: usize, entry: &AdminEntry) {
        if !self.admin_allowed(Caller::Player(client), entry, true) {
            return;
        }
        let Some(system) = self.admin_system() else {
            return;
        };
        if let Some(peer) = self.peer_mut(client) {
            peer.admin.rank = None;
            peer.admin.by_key = false;
        }
        self.admin_tell(client, system.texts.logged_out.as_bytes());
    }

    /// `aminfo` (`Cmd_aminfo_f`): the admin commands the rank may use, then every
    /// player's. JA+ sends a command a line; here the admin commands go in one.
    fn aminfo(&mut self, client: usize) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let rank = self.admin_rank(client);
        let heading = match rank {
            Some(Rank::Rcon | Rank::Council) => Some(system.texts.info_heading[0]),
            Some(Rank::Knight) => Some(system.texts.info_heading[1]),
            Some(Rank::Instructor) => Some(system.texts.info_heading[2]),
            None => None,
        };
        if let Some(heading) = heading {
            self.admin_tell(client, heading.as_bytes());
            let mask = self.admin_mask(system, rank);
            let mut list = Vec::new();
            for bit in 1..=u8::MAX {
                let Some(entry) = system.table.iter().find(|entry| entry.bit == Some(bit)) else {
                    break;
                };
                if rules::allows(rank, mask, entry) {
                    list.extend_from_slice(entry.name.as_bytes());
                    list.extend_from_slice(b", ");
                }
            }
            self.admin_tell(client, &list);
        }
        let lines = system.texts.info_player.concat();
        self.admin_tell(client, lines.as_bytes());
    }

    /// `admGrant <player> <council|knight|instructor|no>` (`Svcmd_admGrant_f`).
    fn adm_grant(&mut self, words: &[&[u8]]) {
        let (Some(&who), Some(&level)) = (words.get(1), words.get(2)) else {
            return;
        };
        let Some(target) = self.admin_valid(self.admin_client_for(who)) else {
            return;
        };
        let rank = if level.eq_ignore_ascii_case(b"no") {
            None
        } else {
            match Rank::from_name(level) {
                Some(rank) if rank != Rank::Rcon => Some(rank),
                _ => {
                    self.admin_reply(
                        Caller::Console,
                        b"^5Usage:  admGrant <ID player> <level> where <level> = council, knight, instructor or no\n",
                    );
                    return;
                }
            }
        };
        let Some(peer) = self.peer_mut(target) else {
            return;
        };
        peer.admin.rank = rank;
        peer.admin.by_key = false;
        let name = peer.name.clone();
        let text = match rank {
            None => [&b"^5Remove Admin access to User ^7"[..], &name, b"\n"].concat(),
            Some(rank) => {
                let mut title = rank.name().to_owned();
                title[..1].make_ascii_uppercase();
                [
                    b"^5Granted ".as_slice(),
                    title.as_bytes(),
                    b" access to User ^7",
                    &name,
                    b"\n",
                ]
                .concat()
            }
        };
        self.admin_reply(Caller::Console, &text);
    }

    /// The name of mask bit `bit` for `admToggle` (0 is VisionJK's flood exemption,
    /// which SJK does not have; it keeps its place).
    fn admin_bit_name(system: &AdminSystem, bit: u8) -> &'static str {
        if bit == 0 {
            return "amFloodExempt";
        }
        system
            .table
            .iter()
            .find(|entry| entry.bit == Some(bit))
            .map_or("", |entry| entry.name)
    }

    /// `admToggle <bit> <rank>` (`Svcmd_admToggle_f`): one bit of a rank's mask flipped,
    /// told to everyone. A mask holds bits 0 to 31.
    fn adm_toggle(&mut self, words: &[&[u8]]) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let usage = b"^5Usage:  ^7admToggle < ID Cmd> <level> where <level> = council, knight, instructor\n\ttype admToggle without argument to display help\n";
        let (Some(&number), Some(&level)) = (words.get(1), words.get(2)) else {
            let mut help = b"^5Usage:  ^7admToggle < ID Cmd> <level> where <level> = council, knight, instructor\n\t\t^5<ID Cmd>  ^7<=> ^5Adm Cmd Name  \n".to_vec();
            for bit in 0..=31u8 {
                help.extend_from_slice(
                    format!(
                        "\t\t^5{bit}         ^7<=> ^5{}  \n",
                        Self::admin_bit_name(system, bit)
                    )
                    .as_bytes(),
                );
            }
            self.admin_reply(Caller::Console, &help);
            return;
        };
        let bit = sjk_game_jka::userinfo::atoi(number);
        let rank = Rank::from_name(level).and_then(Rank::named_index);
        let (Some(index), true) = (
            rank,
            (0..=31).contains(&bit) && (bit != 0 || number.first() == Some(&b'0')),
        ) else {
            self.admin_reply(Caller::Console, usage);
            return;
        };
        let name = system.settings.allowed[index];
        let mask = self.cvars.integer(name.as_bytes());
        let was = rules::bit_allowed(mask, bit as u8);
        self.set_cvar(name, &(mask ^ (1 << bit)).to_string());
        let mut title = Rank::NAMED[index].name().to_owned();
        title[..1].make_ascii_uppercase();
        self.told.push(Told::Everyone(print_command(
            format!(
                "^5{} ^7{} for ^6{title}!\n",
                Self::admin_bit_name(system, bit as u8),
                if was { "disallowed" } else { "allowed" }
            )
            .as_bytes(),
        )));
    }

    /// `admBan <player>` (`Svcmd_admBan_f`): its address admin-banned, or no longer.
    fn adm_ban(&mut self, words: &[&[u8]]) {
        let Some(system) = self.admin_system() else {
            return;
        };
        let Some(&who) = words.get(1) else {
            return;
        };
        let Some(target) = self.admin_valid(self.admin_client_for(who)) else {
            return;
        };
        let Some(peer) = self.peer(target) else {
            return;
        };
        let (name, address) = (peer.name.clone(), peer.address.clone());
        if address.is_empty() {
            self.admin_reply(
                Caller::Console,
                &[&b"^5i can't get the IP of ^7"[..], &name, b"\n"].concat(),
            );
            return;
        }
        if !self.admin_banned(&address) {
            if let Some(peer) = self.peer_mut(target) {
                peer.admin.rank = None;
                peer.admin.by_key = false;
            }
            self.admin_ban_address(&address);
            self.admin_reply(
                Caller::Console,
                &[&b"^5Ban Admin access to User ^7"[..], &name, b"\n"].concat(),
            );
            self.admin_tell(target, system.texts.admin_banned.as_bytes());
        } else {
            let host = address
                .split(|byte| *byte == b':')
                .next()
                .unwrap_or_default()
                .to_vec();
            if let Some(list) = self.admin.filter.remove(&host, &mut |_| {}) {
                self.admin_store_bans(&list);
            }
            self.admin_reply(
                Caller::Console,
                &[&b"^5Unban Admin access to User ^7"[..], &name, b"\n"].concat(),
            );
            self.admin_tell(target, system.texts.admin_unbanned.as_bytes());
        }
    }

    /// `admAddIP <mask>`.
    fn adm_add_ip(&mut self, words: &[&[u8]]) {
        let Some(&mask) = words.get(1) else {
            self.admin_reply(Caller::Console, b"Usage:  admAddIP <ip-mask>\n");
            return;
        };
        let mut out = Vec::new();
        let list = self
            .admin
            .filter
            .add(mask, &mut |text| out.extend_from_slice(text));
        self.admin_reply(Caller::Console, &out);
        self.admin_store_bans(&list);
    }

    /// `admRemoveIP <mask|all>`.
    fn adm_remove_ip(&mut self, words: &[&[u8]]) {
        let Some(&mask) = words.get(1) else {
            self.admin_reply(Caller::Console, b"Usage:  admRemoveIP <ip-mask>\n");
            return;
        };
        if mask.eq_ignore_ascii_case(b"all") {
            self.admin.filter = IpFilter::default();
            self.admin_store_bans(b"");
            return;
        }
        let mut out = Vec::new();
        let list = self
            .admin
            .filter
            .remove(mask, &mut |text| out.extend_from_slice(text));
        self.admin_reply(Caller::Console, &out);
        if let Some(list) = list {
            self.admin_store_bans(&list);
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::bridge::Identity;
    use crate::cvars::Cvars;

    /// A mapless server whose start had `g_mod` set to `server_mod`, with players 0 to 3
    /// in it, named `P0` to `P3`, at addresses `10.0.0.<n>`.
    pub(in crate::bridge) fn game(server_mod: &str) -> NativeGame {
        let mut cvars = Cvars::new();
        cvars.set(b"g_mod", server_mod.as_bytes());
        cvars.set(b"g_userinfoValidate", b"0");
        cvars.register_server();
        let identity = Identity {
            hostname: b"test".to_vec(),
            mapname: b"mp/ffa3".to_vec(),
        };
        let mut game = NativeGame::with_cvars(identity, None, 4, 4, cvars).unwrap();
        for (client, origin) in [
            (0, [500.0, 0.0, 0.0]),
            (1, [700.0, 0.0, 0.0]),
            (2, [500.0, 300.0, 0.0]),
            (3, [900.0, 300.0, 0.0]),
        ] {
            let userinfo = format!(r"\name\P{client}\ip\10.0.0.{client}:29070");
            game.connect_client(client, userinfo.as_bytes(), false)
                .unwrap();
            game.begin(client, 0, None);
            let peer = game.peer_mut(client).unwrap();
            peer.state.set_origin(origin);
            peer.address = format!("10.0.0.{client}:29070").into_bytes();
        }
        game.last_frame_time = 1_000;
        game
    }

    /// What `client` was told, as text, since the last look.
    pub(in crate::bridge) fn told(game: &mut NativeGame, client: usize) -> String {
        let mut out = String::new();
        for told in std::mem::take(&mut game.told) {
            match told {
                Told::One(to, text) if to == client => out += &String::from_utf8_lossy(&text),
                Told::Everyone(text) => out += &String::from_utf8_lossy(&text),
                _ => {}
            }
        }
        out
    }

    /// A console line, and what it printed.
    pub(in crate::bridge) fn console(game: &mut NativeGame, line: &str) -> String {
        let mut out = Vec::new();
        let words: Vec<&[u8]> = LegacyTokens::new(line.as_bytes()).collect();
        let taken = game.admin_console_command(&words, game.last_frame_time, &mut |text| {
            out.extend_from_slice(text)
        });
        assert!(taken, "{line}");
        String::from_utf8_lossy(&out).into_owned()
    }

    fn command(game: &mut NativeGame, client: usize, line: &str) -> String {
        game.player_command(client, line.as_bytes(), game.last_frame_time);
        told(game, client)
    }

    #[test]
    fn the_base_game_has_no_admins() {
        let mut game = game("base");
        assert!(game.admin_system().is_none());
        assert!(command(&mut game, 0, "amlogin secret").contains("Unknown command amlogin"));
        assert!(command(&mut game, 0, "aminfo").contains("Unknown command aminfo"));
        assert_eq!(game.admin_rank(0), None);
        let words: Vec<&[u8]> = vec![b"admGrant", b"0", b"council"];
        assert!(!game.admin_console_command(&words, 0, &mut |_| {}));
        assert_eq!(game.admin_rank(0), None);
    }

    #[test]
    fn passwords_count_only_where_the_owner_allows_them() {
        let mut game = game("japlus");
        game.set_cvar("jp_councilPass", "hunter2");
        let said = command(&mut game, 0, "amlogin hunter2");
        assert!(said.contains("No such code"), "{said}");
        assert_eq!(game.admin_rank(0), None);
        game.set_cvar("japlus.adminPasswords", "1");
        let said = command(&mut game, 0, "amlogin hunter2");
        assert!(
            said.contains("P0 ^7is logged as an ^3admin council."),
            "{said}"
        );
        assert_eq!(game.admin_rank(0), Some(Rank::Council));
        // A wrong one takes the rank away, as in JA+.
        let said = command(&mut game, 0, "amlogin nope");
        assert!(said.contains("Bad Password"), "{said}");
        assert_eq!(game.admin_rank(0), None);
        // The unset value is never a password.
        game.set_cvar("jp_knightPass", "NOT SET");
        let said = command(&mut game, 1, "amlogin \"NOT SET\"");
        assert!(said.contains("try to hack me"), "{said}");
        assert_eq!(game.admin_rank(1), None);
    }

    #[test]
    fn the_plugins_own_login_counts_only_with_passwords() {
        let mut game = game("japlus");
        game.set_cvar("jp_knightPass", "pw");
        let userinfo = br"\name\P1\cp_login\pw".to_vec();
        game.peer_mut(1).unwrap().userinfo = userinfo;
        game.admin_userinfo_login(1);
        assert_eq!(game.admin_rank(1), None);
        game.set_cvar("japlus.adminPasswords", "1");
        game.admin_userinfo_login(1);
        assert_eq!(game.admin_rank(1), Some(Rank::Knight));
    }

    #[test]
    fn wrong_tries_make_a_player_wait() {
        let mut game = game("japlus");
        for _ in 0..3 {
            command(&mut game, 2, "amlogin guess");
        }
        let said = command(&mut game, 2, "amlogin guess");
        assert!(said.contains("Too many wrong tries"), "{said}");
        game.last_frame_time += 30_001;
        let said = command(&mut game, 2, "amlogin guess");
        assert!(said.contains("No such code"), "{said}");
    }

    #[test]
    fn logout_info_and_grant() {
        let mut game = game("japlus");
        let said = command(&mut game, 0, "amlogout");
        assert!(said.contains("not allowed to execute"), "{said}");
        let said = console(&mut game, "admGrant 0 knight");
        assert!(
            said.contains("Granted Knight access to User ^7P0"),
            "{said}"
        );
        assert_eq!(game.admin_rank(0), Some(Rank::Knight));
        let said = command(&mut game, 0, "aminfo");
        assert!(said.contains("Admin KNIGHT Commands"), "{said}");
        // JA+'s default knight mask: kick, not ban.
        assert!(
            said.contains("amkick, ") && !said.contains("amban, "),
            "{said}"
        );
        assert!(said.contains("amlogout, ") && said.contains("Player Commands"));
        let said = command(&mut game, 0, "amlogout");
        assert!(said.contains("not an admin any more"), "{said}");
        assert_eq!(game.admin_rank(0), None);
        assert!(console(&mut game, "admGrant 0 rcon").contains("Usage"));
        console(&mut game, "admGrant P1 council");
        assert_eq!(game.admin_rank(1), Some(Rank::Council));
        console(&mut game, "admGrant 1 no");
        assert_eq!(game.admin_rank(1), None);
    }

    #[test]
    fn toggle_flips_a_bit_of_a_ranks_mask() {
        let mut game = game("japlus");
        let before = game.cvars.integer(b"jp_knightAllowedCMD");
        assert!(console(&mut game, "admToggle").contains("amkick"));
        console(&mut game, "admToggle 2 knight");
        assert_eq!(game.cvars.integer(b"jp_knightAllowedCMD"), before ^ 4);
        assert!(told(&mut game, 0).contains("^5amban ^7allowed for ^6Knight!"));
        assert!(console(&mut game, "admToggle 40 knight").contains("Usage"));
    }

    #[test]
    fn an_admin_banned_address_may_not_log_in() {
        let mut game = game("japlus");
        game.set_cvar("japlus.adminPasswords", "1");
        game.set_cvar("jp_instructorPass", "pw");
        console(&mut game, "admBan 3");
        assert_eq!(game.cvars.string(b"jp_banAdminIPs"), b"10.0.0.3 ");
        let said = command(&mut game, 3, "amlogin pw");
        assert!(said.contains("admin-banned"), "{said}");
        assert_eq!(game.admin_rank(3), None);
        assert!(console(&mut game, "admListIP").contains("10.0.0.3"));
        console(&mut game, "admBan 3");
        assert_eq!(game.cvars.string(b"jp_banAdminIPs"), b"");
        command(&mut game, 3, "amlogin pw");
        assert_eq!(game.admin_rank(3), Some(Rank::Instructor));
        // The list is read again as a level starts.
        console(&mut game, "admAddIP 10.0.0.*");
        game.process_admin_bans();
        assert!(game.admin_banned(b"10.0.0.9:1"));
        console(&mut game, "admRemoveIP all");
        assert!(!game.admin_banned(b"10.0.0.9"));
    }

    #[test]
    fn the_rank_outlives_a_map_change() {
        let mut game = game("japlus");
        console(&mut game, "admGrant 2 council");
        game.change_map("mp/ffa5", None, 2_000);
        assert_eq!(game.admin_rank(2), Some(Rank::Council));
    }

    fn dropped(game: &NativeGame) -> Vec<usize> {
        game.told
            .iter()
            .filter_map(|told| match told {
                Told::Drop { client, .. } => Some(*client),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn kicks_and_bans_follow_the_rights_and_are_logged() {
        let mut game = game("japlus");
        // No rank: refused.
        let said = command(&mut game, 0, "amkick 1");
        assert!(said.contains("not allowed"), "{said}");
        console(&mut game, "admGrant 0 knight");
        console(&mut game, "admGrant 1 council");
        // A knight may not kick a council member (jp_admUsableOn 0), nor ban.
        let said = command(&mut game, 0, "amkick 1");
        assert!(said.contains("on this player"), "{said}");
        assert!(command(&mut game, 0, "amban 2").contains("not allowed"));
        game.player_command(0, b"amkick P2", game.last_frame_time);
        assert_eq!(dropped(&game), [2]);
        game.told.clear();
        // The council bans: the address goes into g_banIPs.
        game.player_command(1, b"amban 3", game.last_frame_time);
        assert_eq!(dropped(&game), [3]);
        assert_eq!(game.cvars.string(b"g_banIPs"), b"10.0.0.3 ");
        // The console kicks anybody.
        game.told.clear();
        console(&mut game, "amkick 1");
        assert_eq!(dropped(&game), [1]);
    }

    #[test]
    fn the_abuse_counter_refuses_then_admin_bans() {
        let mut game = game("japlus");
        game.set_cvar("jp_antiAdmAbuseTimer", "60");
        game.set_cvar("jp_antiAdmAbuseCmdsNumber", "1");
        game.set_cvar("jp_antiAdmAbuseNumberB4ban", "1");
        console(&mut game, "admGrant 0 council");
        game.player_command(0, b"amkick 1", game.last_frame_time);
        assert_eq!(dropped(&game), [1]);
        game.told.clear();
        // The second sensible command: JA+ starts with no abuse left, so it bans.
        let said = command(&mut game, 0, "amkick 2");
        assert!(said.contains("admin-banned"), "{said}");
        assert_eq!(game.admin_rank(0), None);
        assert_eq!(game.cvars.string(b"jp_banAdminIPs"), b"10.0.0.0 ");
    }

    #[test]
    fn helpers_fill_texts() {
        assert_eq!(
            fill("^5%s ^7wait %is, %i", &[b"x"], &[3, 4]),
            b"^5x ^7wait 3s, 4"
        );
    }
}
