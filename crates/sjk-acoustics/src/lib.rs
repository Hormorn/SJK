//! SJK's 3D sound acoustics (`s_spatial`): what reaches the listener from each sound
//! of a level and how the room round them rings, worked out from the level's own
//! walls on threads of their own. SJK's own code, written after the ideas of Valve's
//! Steam Audio and of VisionJK's `v_spatial` (Blackwolf), with none of their code.
//!
//! - [`scene`]: the level's solid brushes and curves as triangles with their
//!   [`material`]s, in a [`bvh`], and the doors and lifts where the game shows them.
//! - [`direct`]: per sound, the share in sight, what the walls let through and the
//!   way round a corner.
//! - [`room`]: rays from the head bouncing through the level, turned into each
//!   ear's impulse response.
//! - [`worker`]: the threads, fed a snapshot each frame.
//!
//! The mixer's side (the head's filters, the convolution) is in `sjk-audio`.

pub mod bvh;
pub mod direct;
pub mod material;
pub mod room;
pub mod scene;
pub mod worker;

pub use room::{RoomSettings, RoomStats};
pub use scene::Mover;
pub use worker::{Acoustics, Output, Snapshot};
