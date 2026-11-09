// Sol JK site enhancements. The page works without this script: the download
// button points at the latest release page, What's new links to the changelog,
// Install lists every platform and the credits link each contributor's pull requests.
"use strict";

const REPO = "Sol-Vulpes/SJK";
const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=8`;
const RELEASES_PAGE = `https://github.com/${REPO}/releases`;
// The changelog the client shows in game: What's new and the home page's recap read
// its releases, their introductions and "> Heading: sentence" lines (CHANGELOG.md).
const CHANGELOG_RAW = `https://raw.githubusercontent.com/${REPO}/main/CHANGELOG.md`;
const CHANGELOG_PAGE = `https://github.com/${REPO}/blob/main/CHANGELOG.md`;
/** How many releases What's new lists. */
const RELEASES_SHOWN = 5;
// Merged pull requests per contributor (the credits' `data-author` logins), counted
// by GitHub's search, which allows 10 searches a minute without a token: the counts
// are kept an hour in the visitor's browser.
const SEARCH_API = "https://api.github.com/search/issues";
const COUNTS_KEY = "sjk-site-pr-counts";
const COUNTS_TTL_MS = 60 * 60 * 1000;

/** Release ZIPs by platform, as the release workflow names them. */
// The SJK Launcher's own repository: its latest release is the launcher section's
// download, picked for the visitor's platform as SJK's is.
const LAUNCHER_REPO = "Sol-Vulpes/sjk-launcher";
const LAUNCHER_API = `https://api.github.com/repos/${LAUNCHER_REPO}/releases/latest`;

const PLATFORMS = {
  "windows-x64": { system: "Windows", label: "Windows 10/11 x64" },
  "linux-x64": { system: "Linux", label: "Linux x64" },
  "macos-arm64": { system: "Mac", label: "Mac with Apple Silicon (experimental)" },
};

// Donations (docs/sjk.md, "Donations"). The card reads the live goal from the SJK hub
// (`hub`, which Ko-fi's webhook keeps up to date); the other values are what it shows
// when the hub cannot be reached: `url` the donation page (an https address; anything
// else says the link is coming), `goal` the monthly goal and `raised` what came in
// this month, both whole euros.
const DONATION = {
  // Donations are shelved (14/10/2026): the section stays hidden and the hub is not
  // asked. Set to false along with removing index.html's hidden attributes.
  shelved: true,
  hub: "https://sjk.dfox.app/v1/donations/goal",
  url: "https://ko-fi.com/soljk",
  goal: 200,
  raised: 0,
  monthly: true,
};

/** Escape text for safe insertion as HTML. */
function escapeHtml(text) {
  return String(text).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

/** Inline markdown on already-escaped text: code, bold, links, bare URLs. */
function inline(escaped) {
  const codes = [];
  let text = escaped.replace(/`([^`]+)`/g, (_, code) => {
    codes.push(`<code>${code}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  text = text
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, (_, lead, url) => {
      const label = url.replace(/^https:\/\/github\.com\/[^/]+\/[^/]+\/pull\//, "#");
      return `${lead}<a href="${url}">${label}</a>`;
    });
  return text.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[Number(i)]);
}

/** The newest published, non-prerelease release, or the newest of any kind. */
function latestRelease(releases) {
  const published = releases.filter((r) => !r.draft);
  return published.find((r) => !r.prerelease) || published[0] || null;
}

/** Download asset for a platform ("windows-x64", "linux-x64" or "macos-arm64"). */
function assetFor(release, platform) {
  return (release.assets || []).find((a) => a.name.endsWith(`-${platform}.zip`)) || null;
}

/** The launcher's archive for a platform, as its build workflow names them
 * (`sjk-launcher-windows-x64.zip`, `sjk-launcher-linux-x64.tar.gz`…). */
function launcherAssetFor(release, platform) {
  return (release.assets || []).find((a) => a.name.startsWith(`sjk-launcher-${platform}.`)) || null;
}

function versionOf(release) {
  return (release.tag_name || "").replace(/^sjk-v/, "") || release.name || "";
}

/** The release page of a changelog `version`. */
function tagUrl(version) {
  return `https://github.com/${REPO}/releases/tag/sjk-v${encodeURIComponent(version)}`;
}

/** The visitor's platform from the browser's description of itself; Windows when it
 * names none SJK builds for (phones and tablets included). */
function guessPlatform(
  agent = `${navigator.userAgent} ${navigator.platform || ""}`,
  touchPoints = navigator.maxTouchPoints || 0,
) {
  if (/Android|iPhone|iPad/i.test(agent)) return "windows-x64";
  // An iPad asking for the desktop site says Macintosh too, but has a touch screen.
  if (/Macintosh|Mac OS X|MacIntel/i.test(agent)) return touchPoints > 1 ? "windows-x64" : "macos-arm64";
  if (/Linux|X11/i.test(agent)) return "linux-x64";
  return "windows-x64";
}

/** The download button for `platform`, the other platforms' ZIPs under it. */
function enhanceDownload(release, platform) {
  const button = document.getElementById("download");
  if (!button || !release) return;
  const version = versionOf(release);
  const asset = assetFor(release, platform);
  button.href = asset ? asset.browser_download_url : release.html_url;
  const label = document.getElementById("download-label");
  if (label) label.textContent = asset ? `Download for ${PLATFORMS[platform].system}` : "Download";
  const line = document.getElementById("download-line");
  if (line) line.textContent = asset ? `Sol JK ${version} · ${PLATFORMS[platform].label}` : `Sol JK ${version}`;
  const others = document.getElementById("download-others");
  if (others) {
    const links = Object.keys(PLATFORMS)
      .filter((p) => !asset || p !== platform)
      .map((p) => [p, assetFor(release, p)])
      .filter(([, a]) => a)
      .map(([p, a]) => `<a href="${escapeHtml(a.browser_download_url)}">${PLATFORMS[p].label}</a>`);
    links.push(`<a href="${escapeHtml(release.html_url)}">all files and checksums</a>`);
    others.innerHTML = links.join(" · ");
  }
}

/** The launcher section's download button for `platform`, the other platforms'
 * archives under it. Without an answer from GitHub it keeps the releases page. */
async function loadLauncher(platform) {
  const button = document.getElementById("launcher-download");
  if (!button) return;
  let release;
  try {
    release = await fetchJson(LAUNCHER_API);
  } catch (error) {
    console.info("Launcher release unavailable:", error);
    return;
  }
  if (!release || !release.html_url) return;
  const version = (release.tag_name || "").replace(/^v/, "");
  const asset = launcherAssetFor(release, platform);
  button.href = asset ? asset.browser_download_url : release.html_url;
  const label = document.getElementById("launcher-download-label");
  if (label && asset) label.textContent = `Get it for ${PLATFORMS[platform].system}`;
  const line = document.getElementById("launcher-line");
  if (line) line.textContent = asset ? `SJK Launcher ${version} · ${PLATFORMS[platform].label}` : `SJK Launcher ${version}`;
  const others = document.getElementById("launcher-others");
  if (others) {
    const links = Object.keys(PLATFORMS)
      .filter((p) => !asset || p !== platform)
      .map((p) => [p, launcherAssetFor(release, p)])
      .filter(([, a]) => a)
      .map(([p, a]) => `<a href="${escapeHtml(a.browser_download_url)}">${PLATFORMS[p].label}</a>`);
    links.push(`<a href="${escapeHtml(release.html_url)}">all files and checksums</a>`);
    others.innerHTML = links.join(" · ");
  }
}

/** The launcher's pictures: each card shows its screenshot in the big frame
 * instead of opening the file. */
function setUpLauncherShots() {
  const frame = document.getElementById("launcher-shot");
  const caption = document.getElementById("launcher-caption");
  const cards = [...document.querySelectorAll("[data-launcher-shot]")];
  if (!frame || !cards.length) return;
  const show = (card) => {
    cards.forEach((c) => c.setAttribute("aria-current", String(c === card)));
    frame.src = card.getAttribute("href");
    frame.alt = card.querySelector("img").alt;
    if (caption) {
      caption.innerHTML = `<strong>${escapeHtml(card.querySelector("strong").textContent)}</strong>${escapeHtml(card.dataset.launcherShot)}`;
    }
  };
  cards.forEach((card) => card.addEventListener("click", (event) => {
    event.preventDefault();
    show(card);
    if (frame.getBoundingClientRect().top < 0) frame.scrollIntoView({ behavior: reducedMotion ? "auto" : "smooth", block: "start" });
  }));
}

/** The changelog's releases, newest first, without Unreleased: each with its
 * heading, version, stage ("Alpha"), date (dd/mm/yyyy), introduction, recap lines
 * ({heading, text}) and number of changes. */
function parseChangelog(source) {
  const releases = [];
  let current = null;
  let comment = false;
  for (const raw of String(source || "").replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trim();
    if (comment) {
      if (line.includes("-->")) comment = false;
      continue;
    }
    if (line.startsWith("<!--")) {
      comment = !line.includes("-->");
      continue;
    }
    const heading = line.match(/^##\s+(.+)$/);
    if (heading) {
      const [title, date = ""] = heading[1].split("|").map((part) => part.trim());
      const version = title.replace(/\s*\([^)]*\)\s*$/, "");
      current = /^unreleased$/i.test(version) ? null : {
        heading: heading[1],
        version,
        stage: (title.match(/\(([^)]+)\)\s*$/) || [])[1] || "",
        date,
        intro: [],
        recap: [],
        changes: 0,
      };
      if (current) releases.push(current);
      continue;
    }
    if (!current) continue;
    const recap = line.match(/^>\s*([^:]+):\s*(.+)$/);
    if (recap) current.recap.push({ heading: recap[1].trim(), text: recap[2].trim() });
    else if (/^- /.test(line)) current.changes += 1;
    else if (line && !line.startsWith("=") && !current.changes && !current.recap.length) current.intro.push(line);
  }
  return releases.map((r) => ({ ...r, intro: r.intro.join(" ") }));
}

/** GitHub's anchor for a changelog heading: lower case, punctuation dropped, each
 * space a hyphen. */
function changelogAnchor(heading) {
  return String(heading).toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, "").replace(/\s/g, "-");
}

/** A dd/mm/yyyy date as yyyy-mm-dd, or "" when it is not one. */
function isoDate(date) {
  const parts = String(date).match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  return parts ? `${parts[3]}-${parts[2]}-${parts[1]}` : "";
}

/** A recap line, its heading in gold or holo blue by the stylesheet. */
function recapItem(line) {
  return `<li><strong>${escapeHtml(line.heading)}</strong> <span>${inline(escapeHtml(line.text))}</span></li>`;
}

/** One release of What's new. */
function releaseItem(entry) {
  const when = entry.date
    ? `<time${isoDate(entry.date) ? ` datetime="${isoDate(entry.date)}"` : ""}>${escapeHtml(entry.date)}</time>`
    : "";
  const intro = entry.intro ? `<p class="log-intro">${inline(escapeHtml(entry.intro))}</p>` : "";
  const recap = entry.recap.length ? `<ul class="log-recap">${entry.recap.map(recapItem).join("")}</ul>` : "";
  const more = entry.changes
    ? `<p class="log-more"><a href="${CHANGELOG_PAGE}#${changelogAnchor(entry.heading)}">${
      entry.changes === 1 ? "The change in full" : `All ${entry.changes} changes`}</a></p>`
    : "";
  return `<li>
      <div class="log-head"><h3><a href="${tagUrl(entry.version)}">${escapeHtml(entry.version)}</a></h3>${when}</div>
      ${intro}${recap}${more}
    </li>`;
}

/** What's new from the changelog's `entries`, only those GitHub has published when
 * `releases` is known (a release's notes are written before its tag). */
function renderChangelog(entries, releases) {
  const target = document.getElementById("releases");
  if (!target) return;
  const tags = releases && releases.length
    ? new Set(releases.filter((r) => !r.draft).map((r) => r.tag_name))
    : null;
  const shown = entries.filter((e) => !tags || tags.has(`sjk-v${e.version}`)).slice(0, RELEASES_SHOWN);
  if (!shown.length) return;
  target.innerHTML = `<ol class="log">${shown.map(releaseItem).join("")}</ol>
    <p class="log-all"><a href="${RELEASES_PAGE}">All releases</a> · <a href="${CHANGELOG_PAGE}">The full changelog</a></p>`;
}

/** The home page's latest release: its changelog `entry`'s recap, else GitHub's
 * `release` name and date. */
function showLatest(entry, release) {
  const name = document.getElementById("release-name");
  const date = document.getElementById("release-date");
  const recap = document.getElementById("release-recap");
  if (entry) {
    const title = entry.stage ? `${entry.version} (${entry.stage})` : entry.version;
    if (name) name.innerHTML = `<a href="${tagUrl(entry.version)}">${escapeHtml(title)}</a>`;
    if (date) date.textContent = entry.date;
    if (recap && entry.recap.length) recap.innerHTML = entry.recap.slice(0, 6).map(recapItem).join("");
    else if (recap && entry.intro) recap.innerHTML = `<li><span>${inline(escapeHtml(entry.intro))}</span></li>`;
  } else if (release) {
    if (name) name.innerHTML = `<a href="${escapeHtml(release.html_url)}">${escapeHtml(release.name || versionOf(release))}</a>`;
    if (date && release.published_at) date.textContent = euDate(release.published_at);
  }
}

/** `url`'s answer as text, given up after `seconds`. */
async function fetchText(url, options = {}, seconds = 8) {
  const abort = typeof AbortController === "function" ? new AbortController() : null;
  const timer = abort ? setTimeout(() => abort.abort(), seconds * 1000) : null;
  try {
    const response = await fetch(url, abort ? { ...options, signal: abort.signal } : options);
    if (!response.ok) throw new Error(`${url}: ${response.status}`);
    return await response.text();
  } finally {
    if (timer) clearTimeout(timer);
  }
}

async function fetchJson(url) {
  return JSON.parse(await fetchText(url, { headers: { Accept: "application/vnd.github+json" } }));
}

/** The latest release's download, the home page's recap and What's new. */
async function loadReleases(platform) {
  const [releases, changelog] = await Promise.all([
    fetchJson(RELEASES_API).then((r) => (Array.isArray(r) ? r : null)).catch((error) => {
      console.info("Release information unavailable:", error);
      return null;
    }),
    fetchText(CHANGELOG_RAW).catch((error) => {
      console.info("Changelog unavailable:", error);
      return null;
    }),
  ]);
  // Without either, the static links stay: they always work.
  const latest = releases ? latestRelease(releases) : null;
  if (latest) enhanceDownload(latest, platform);
  const entries = changelog ? parseChangelog(changelog) : [];
  renderChangelog(entries, releases);
  const entry = latest
    ? entries.find((e) => `sjk-v${e.version}` === latest.tag_name)
    : entries[0];
  showLatest(entry || null, latest);
}

/** Merged pull requests per GitHub login among search results' `items`. */
function countByAuthor(items) {
  const counts = {};
  for (const item of items || []) {
    const login = item && item.user && item.user.login;
    if (login) counts[login] = (counts[login] || 0) + 1;
  }
  return counts;
}

function cachedCounts(authors) {
  try {
    const kept = JSON.parse(localStorage.getItem(COUNTS_KEY) || "null");
    if (!kept || Date.now() - kept.at > COUNTS_TTL_MS) return null;
    return authors.every((a) => typeof kept.counts[a] === "number") ? kept.counts : null;
  } catch {
    return null;
  }
}

/** The merged pull requests of each of `authors` in SJK: one search for all of them,
 * a page of 100 at a time. */
async function contributorCounts(authors) {
  const cached = cachedCounts(authors);
  if (cached) return cached;
  const query = [`repo:${REPO}`, "is:pr", "is:merged", ...authors.map((a) => `author:${a}`)].join(" ");
  const items = [];
  for (let page = 1; page <= 10; page += 1) {
    const answer = await fetchJson(`${SEARCH_API}?q=${encodeURIComponent(query)}&per_page=100&page=${page}`);
    const found = Array.isArray(answer.items) ? answer.items : [];
    items.push(...found);
    if (!found.length || items.length >= Number(answer.total_count)) break;
  }
  const counts = countByAuthor(items);
  for (const author of authors) counts[author] = counts[author] || 0;
  try {
    localStorage.setItem(COUNTS_KEY, JSON.stringify({ at: Date.now(), counts }));
  } catch {
    // No storage: the next visit asks GitHub again.
  }
  return counts;
}

/** Each contributor card's count of merged pull requests. */
async function loadContributors() {
  const cards = [...document.querySelectorAll("[data-author]")];
  if (!cards.length) return;
  let counts;
  try {
    counts = await contributorCounts(cards.map((card) => card.dataset.author));
  } catch (error) {
    // The cards keep "Pull requests" and their links.
    console.info("Pull request counts unavailable:", error);
    return;
  }
  for (const card of cards) {
    const count = counts[card.dataset.author];
    const line = card.querySelector(".count");
    if (line && count) line.textContent = count === 1 ? "1 merged pull request" : `${count} merged pull requests`;
  }
}

/** Install shows one platform at a time, the visitor's first, chosen by its buttons. */
function setUpPlatforms(initial) {
  const root = document.querySelector("[data-platforms]");
  if (!root) return;
  const pick = root.querySelector(".platform-pick");
  const buttons = [...root.querySelectorAll("[data-platform]")];
  const sections = [...root.querySelectorAll(".platform[data-for]")];
  if (!pick || !buttons.length || !sections.length) return;
  const show = (platform) => {
    buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.platform === platform)));
    sections.forEach((s) => s.classList.toggle("shown", s.dataset.for === platform));
  };
  buttons.forEach((b) => b.addEventListener("click", () => show(b.dataset.platform)));
  root.classList.add("picked");
  pick.hidden = false;
  show(sections.some((s) => s.dataset.for === initial) ? initial : sections[0].dataset.for);
}

/** A whole number of euros as SJK's site writes them: "1 200 €". */
function euros(amount) {
  const whole = Math.max(0, Math.round(Number(amount) || 0));
  const narrowSpace = String.fromCharCode(0x202f);
  const noBreak = String.fromCharCode(0xa0);
  return `${String(whole).replace(/\B(?=(\d{3})+(?!\d))/g, narrowSpace)}${noBreak}€`;
}

/** What the donation card shows for `donation`: the amount line, the bar's fill
 * (0 to 1) and the link, or null while the link is not an https address. */
function donationState(donation) {
  const goal = Math.max(1, Math.round(Number(donation.goal) || 0));
  const raised = Math.max(0, Math.round(Number(donation.raised) || 0));
  const url = /^https:\/\/\S+$/.test(String(donation.url || "")) ? donation.url : null;
  const period = donation.monthly === false ? "" : " this month";
  return {
    amount: `${euros(raised)} of ${euros(goal)}${period}`,
    fill: Math.min(1, raised / goal),
    goal,
    raised,
    url,
  };
}

/** The hub's goal answer as DONATION's fields, or null when it is not one. */
function donationFromHub(answer) {
  if (!answer || !(Number(answer.goal_cents) > 0)) return null;
  const url = /^https:\/\/\S+$/.test(String(answer.url || "")) ? answer.url : DONATION.url;
  return {
    url,
    goal: Number(answer.goal_cents) / 100,
    raised: Math.max(0, Number(answer.raised_cents) || 0) / 100,
    monthly: answer.period !== "total",
  };
}

/** The donation card from DONATION at once, then from the hub's live goal. */
async function setUpDonation() {
  if (DONATION.shelved) return;
  showDonation(DONATION);
  if (!DONATION.hub || typeof fetch !== "function") return;
  const abort = typeof AbortController === "function" ? new AbortController() : null;
  const timer = abort ? setTimeout(() => abort.abort(), 6000) : null;
  try {
    const response = await fetch(DONATION.hub, abort ? { signal: abort.signal } : {});
    if (!response.ok) throw new Error(`hub ${response.status}`);
    const live = donationFromHub(await response.json());
    if (live) showDonation(live);
  } catch (error) {
    // Keep DONATION's values; the link still works.
    console.info("Donation goal unavailable:", error);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/** Show `donation` on the card: the goal, the bar and the button. */
function showDonation(donation) {
  const amount = document.getElementById("donation-amount");
  const bar = document.getElementById("donation-bar");
  const note = document.getElementById("donation-note");
  const link = document.getElementById("donation-link");
  if (!amount || !bar || !link) return;
  const state = donationState(donation);
  amount.textContent = state.amount;
  bar.hidden = false;
  bar.setAttribute("aria-valuemin", "0");
  bar.setAttribute("aria-valuemax", String(state.goal));
  bar.setAttribute("aria-valuenow", String(Math.min(state.raised, state.goal)));
  bar.style.setProperty("--fill", String(state.fill));
  if (note) note.hidden = donation.monthly === false;
  link.hidden = false;
  if (state.url) {
    link.href = state.url;
    link.rel = "noopener";
    link.textContent = "Donate on Ko-fi";
    link.classList.remove("waiting");
    link.removeAttribute("aria-disabled");
  } else {
    link.textContent = "Donation link coming soon";
    link.classList.add("waiting");
    link.setAttribute("aria-disabled", "true");
  }
}

/** A date as SJK writes them for people: dd/mm/yyyy, in the reader's time zone. */
function euDate(iso) {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const two = (n) => String(n).padStart(2, "0");
  return `${two(date.getDate())}/${two(date.getMonth() + 1)}/${date.getFullYear()}`;
}

async function loadGallery() {
  const section = document.getElementById("screenshots");
  const gallery = document.getElementById("gallery");
  const dialog = document.getElementById("lightbox");
  if (!section || !gallery) return;
  let shots;
  try {
    const response = await fetch("screenshots/manifest.json", { cache: "no-cache" });
    if (!response.ok) return;
    shots = (await response.json()).filter((s) => s && typeof s.file === "string");
  } catch {
    return;
  }
  if (!shots.length) return;
  gallery.innerHTML = shots.map((s, i) => `<button type="button" class="shot" data-index="${i}">
      <img src="screenshots/${encodeURI(s.file)}" alt="${escapeHtml(s.caption || "SJK screenshot")}" loading="lazy" decoding="async">
      ${s.caption ? `<span>${escapeHtml(s.caption)}</span>` : ""}
    </button>`).join("\n");
  section.hidden = false;
  document.querySelectorAll("[data-gallery-link]").forEach((el) => { el.hidden = false; });

  if (!dialog || typeof dialog.showModal !== "function") {
    gallery.addEventListener("click", (event) => {
      const button = event.target.closest(".shot");
      if (button) window.open(`screenshots/${encodeURI(shots[Number(button.dataset.index)].file)}`, "_blank", "noopener");
    });
    return;
  }
  const image = document.getElementById("lightbox-image");
  const caption = document.getElementById("lightbox-caption");
  let current = 0;
  const show = (index) => {
    current = (index + shots.length) % shots.length;
    image.src = `screenshots/${encodeURI(shots[current].file)}`;
    image.alt = shots[current].caption || "SJK screenshot";
    caption.textContent = shots[current].caption || "";
  };
  gallery.addEventListener("click", (event) => {
    const button = event.target.closest(".shot");
    if (!button) return;
    show(Number(button.dataset.index));
    dialog.showModal();
  });
  dialog.addEventListener("click", (event) => {
    const step = event.target.closest("[data-step]");
    if (step) show(current + Number(step.dataset.step));
    else if (event.target.closest("[data-close]") || event.target === dialog) dialog.close();
  });
  dialog.addEventListener("keydown", (event) => {
    if (event.key === "ArrowRight") show(current + 1);
    if (event.key === "ArrowLeft") show(current - 1);
  });
}

const reducedMotion = typeof window !== "undefined"
  && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Behind the home page, shots of the client's camera tour of Yavin Training
 * Grounds, 15 seconds each, crossing through the navy ground. */
function setUpBackdrop() {
  const shots = [...document.querySelectorAll(".backdrop img")];
  if (shots.length < 2 || reducedMotion) return;
  let current = 0;
  setInterval(() => {
    if (document.hidden) return;
    const next = shots[(current + 1) % shots.length];
    const show = () => {
      shots[current].classList.remove("shown");
      next.classList.add("shown");
      current = shots.indexOf(next);
    };
    if (next.dataset.src) {
      next.addEventListener("load", show, { once: true });
      next.src = next.dataset.src;
      delete next.dataset.src;
    } else {
      show();
    }
  }, 15000);
}

/** The top line is clear over the home page and solid below it, and marks the
 * section being read. */
function setUpNav() {
  const nav = document.querySelector(".nav");
  const home = document.querySelector(".home");
  if (!nav || !home || !("IntersectionObserver" in window)) {
    if (nav) nav.classList.remove("over-home");
    return;
  }
  new IntersectionObserver(([entry]) => {
    nav.classList.toggle("over-home", entry.isIntersecting);
  }, { rootMargin: "-72px 0px 0px 0px" }).observe(home);
  const links = new Map([...nav.querySelectorAll("ul a")].map((a) => [a.getAttribute("href").slice(1), a]));
  const spy = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) links.forEach((a, id) => a.classList.toggle("chosen", id === entry.target.id));
    }
  }, { rootMargin: "-40% 0px -55% 0px" });
  links.forEach((_, id) => {
    const section = document.getElementById(id);
    if (section) spy.observe(section);
  });
}

/** What SJK brings, as the client's Settings: one category on show, chosen
 * on the rail by a click or the arrow keys. Without the script all are listed. */
function setUpSettings() {
  const settings = document.querySelector("[data-settings]");
  if (!settings) return;
  const rail = settings.querySelector(".rail");
  const tabs = [...rail.querySelectorAll("a")];
  const cats = tabs.map((a) => document.getElementById(a.getAttribute("href").slice(1)));
  if (cats.some((c) => !c)) return;
  settings.classList.add("tabbed");
  rail.setAttribute("role", "tablist");
  const show = (index, focus) => {
    tabs.forEach((tab, i) => {
      const chosen = i === index;
      tab.classList.toggle("chosen", chosen);
      tab.setAttribute("role", "tab");
      tab.setAttribute("aria-selected", String(chosen));
      tab.tabIndex = chosen ? 0 : -1;
      cats[i].setAttribute("role", "tabpanel");
      cats[i].classList.toggle("shown", chosen);
    });
    if (focus) tabs[index].focus();
  };
  tabs.forEach((tab, i) => {
    tab.addEventListener("click", (event) => {
      event.preventDefault();
      show(i, false);
    });
  });
  rail.addEventListener("keydown", (event) => {
    const at = tabs.indexOf(document.activeElement);
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
    if (at < 0 || !step) return;
    event.preventDefault();
    show((at + step + tabs.length) % tabs.length, true);
  });
  show(0, false);
}

if (typeof document !== "undefined") {
  document.documentElement.classList.add("js");
  document.addEventListener("DOMContentLoaded", () => {
    const platform = guessPlatform();
    setUpBackdrop();
    setUpNav();
    setUpSettings();
    setUpPlatforms(platform);
    setUpDonation();
    loadReleases(platform);
    setUpLauncherShots();
    loadLauncher(platform);
    loadGallery();
    loadContributors();
  });
}

if (typeof module !== "undefined") {
  module.exports = {
    escapeHtml, inline, latestRelease, assetFor, launcherAssetFor, versionOf, euDate, guessPlatform,
    parseChangelog, changelogAnchor, isoDate, countByAuthor, euros, donationState,
    donationFromHub, DONATION, PLATFORMS,
  };
}
