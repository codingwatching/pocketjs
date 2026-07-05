//! `pocket3d-anim` — skinned skeletal animation runtime (DESIGN.md §17).
//!
//! This crate is **pure runtime math** over [`pocket3d_core`] types. It performs
//! no asset I/O and has no knowledge of glTF — decoding `.glb`/`.p3danim` files
//! into the runtime types defined here is the job of `pocket3d-assets` (DESIGN.md
//! §17 asset path). Given an already-decoded [`Skeleton`] and one or more
//! [`AnimationClip`]s, this crate:
//!
//! 1. samples a clip at a point in time into a [`Pose`] (per-joint *local*
//!    transforms), interpolating keyframes (DESIGN.md §17 "clip sampling");
//! 2. blends poses for crossfades (DESIGN.md §16, ~150 ms bot transitions);
//! 3. evaluates the joint hierarchy into a skinning palette via
//!    [`compute_joint_matrices`], the `Vec<Mat4>` that
//!    `pocket3d_render::SkinnedInstance::joint_matrices` expects (DESIGN.md §17
//!    "joint matrix upload").
//!
//! ## Conventions
//!
//! * Joints are stored **topologically sorted**: a joint's parent always has a
//!   lower index than the joint itself. glTF skins satisfy this after a depth-
//!   first flatten, and the single forward pass in [`Skeleton::bind_model_matrices`]
//!   / [`compute_joint_matrices`] relies on it.
//! * A *skinning matrix* is `model_space_joint_matrix * joint.inverse_bind`. At
//!   the bind pose `model == bind`, so every skinning matrix is the identity and
//!   vertices stay put.
//! * All transforms use [`pocket3d_core`]'s Z-up, right-handed world (DESIGN.md §8).

use pocket3d_core::{Mat4, Quat, Transform, Vec3, Vec4};

/// A single joint (bone) in a [`Skeleton`] (DESIGN.md §17: joint hierarchy +
/// inverse bind matrices).
#[derive(Clone, Debug)]
pub struct Joint {
    /// Human-readable joint name (as authored in the source rig).
    pub name: String,
    /// Index of the parent joint in [`Skeleton::joints`], or `None` for a root.
    /// Parents MUST precede their children (topologically sorted).
    pub parent: Option<usize>,
    /// Parent-relative (local) transform in the rest/bind pose.
    pub local_bind: Transform,
    /// Inverse of this joint's model-space bind matrix (model→joint at bind).
    /// Multiplying an animated model-space joint matrix by this yields the
    /// skinning matrix uploaded to the GPU.
    pub inverse_bind: Mat4,
}

/// A joint hierarchy plus per-joint inverse bind matrices (DESIGN.md §17).
#[derive(Clone, Debug, Default)]
pub struct Skeleton {
    /// Joints in topological order (each parent precedes its children).
    pub joints: Vec<Joint>,
}

impl Skeleton {
    /// Number of joints in the skeleton.
    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Per-joint **local→model** bind matrices, walking parents so children
    /// accumulate their ancestors' transforms (DESIGN.md §17). Because joints
    /// are topologically sorted, a single forward pass suffices.
    ///
    /// A correct set of [`Joint::inverse_bind`] matrices is simply the inverse
    /// of each entry here.
    pub fn bind_model_matrices(&self) -> Vec<Mat4> {
        let mut model = Vec::with_capacity(self.joints.len());
        for joint in &self.joints {
            let local = joint.local_bind.matrix();
            let m = match joint.parent {
                Some(p) => model[p] * local,
                None => local,
            };
            model.push(m);
        }
        model
    }

    /// The bind pose expressed as per-joint local transforms. This is the base a
    /// clip starts from before its channels override individual joints.
    pub fn bind_pose(&self) -> Pose {
        Pose {
            locals: self.joints.iter().map(|j| j.local_bind).collect(),
        }
    }
}

/// The property an animation [`Channel`] drives on its target joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKind {
    /// Local translation (packed in the value's `.xyz`).
    Translation,
    /// Local rotation (packed as a quaternion `xyzw`).
    Rotation,
    /// Local scale (packed in the value's `.xyz`).
    Scale,
}

/// One animation track: a stream of keyframes driving a single property of a
/// single joint (DESIGN.md §17 "animation channels").
///
/// Values are stored uniformly as [`Vec4`]:
/// * [`ChannelKind::Rotation`] packs a quaternion as `xyzw`;
/// * [`ChannelKind::Translation`] / [`ChannelKind::Scale`] pack the vector in
///   `.xyz` (`.w` unused).
#[derive(Clone, Debug)]
pub struct Channel {
    /// Index into [`Skeleton::joints`] this channel animates.
    pub target_joint: usize,
    /// Which local property the channel drives.
    pub kind: ChannelKind,
    /// Keyframe times in seconds, strictly increasing.
    pub times: Vec<f32>,
    /// Keyframe values, one per entry in `times` (see struct docs for packing).
    pub values: Vec<Vec4>,
}

impl Channel {
    /// Sample the channel at `time` (already clamped/wrapped to the clip range)
    /// and write the interpolated value into `out`. Times before the first or
    /// after the last keyframe hold the endpoint (clamped). Translation/scale
    /// use LERP; rotation uses SLERP along the shortest arc (DESIGN.md §17).
    fn sample_into(&self, time: f32, out: &mut Transform) {
        if self.times.is_empty() {
            return;
        }
        // Hold before the first keyframe.
        if time <= self.times[0] {
            self.write(out, self.values[0]);
            return;
        }
        // Hold after the last keyframe.
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            self.write(out, self.values[last]);
            return;
        }
        // Locate the segment [i, i+1] with times[i] <= time < times[i+1].
        let mut i = 0;
        while i + 1 < self.times.len() && self.times[i + 1] <= time {
            i += 1;
        }
        let (t0, t1) = (self.times[i], self.times[i + 1]);
        let span = t1 - t0;
        let alpha = if span > 0.0 { (time - t0) / span } else { 0.0 };
        let (a, b) = (self.values[i], self.values[i + 1]);

        match self.kind {
            ChannelKind::Translation => {
                out.translation = a.truncate().lerp(b.truncate(), alpha);
            }
            ChannelKind::Scale => {
                out.scale = a.truncate().lerp(b.truncate(), alpha);
            }
            ChannelKind::Rotation => {
                // glam's `slerp` normalizes and picks the shortest arc.
                out.rotation = Quat::from_vec4(a).slerp(Quat::from_vec4(b), alpha);
            }
        }
    }

    /// Overwrite the relevant field of `out` with a raw keyframe value.
    fn write(&self, out: &mut Transform, v: Vec4) {
        match self.kind {
            ChannelKind::Translation => out.translation = v.truncate(),
            ChannelKind::Scale => out.scale = v.truncate(),
            ChannelKind::Rotation => out.rotation = Quat::from_vec4(v).normalize(),
        }
    }
}

/// A named animation clip: a bundle of [`Channel`]s with a duration (DESIGN.md §17).
#[derive(Clone, Debug)]
pub struct AnimationClip {
    /// Clip name (e.g. `"idle"`, `"walk"`, `"death"`).
    pub name: String,
    /// Clip length in seconds.
    pub duration: f32,
    /// The tracks that make up this clip.
    pub channels: Vec<Channel>,
}

impl AnimationClip {
    /// Sample the clip at `time` seconds into a full [`Pose`] (DESIGN.md §17
    /// "pose evaluation").
    ///
    /// The pose starts from `skeleton`'s bind pose so joints without a channel
    /// keep their bind-local transform, then each channel overrides its target
    /// joint. `looping` wraps `time` by [`AnimationClip::duration`]; otherwise
    /// `time` is clamped to `[0, duration]` (holding the last frame).
    ///
    /// Note: this takes `skeleton` (in addition to the `time, looping`
    /// arguments named in the design) because a full pose is defined as "one
    /// local transform per skeleton joint, starting from the bind pose" — the
    /// clip alone does not know the joint count or the bind pose.
    pub fn sample(&self, skeleton: &Skeleton, time: f32, looping: bool) -> Pose {
        let t = self.resolve_time(time, looping);
        let mut pose = skeleton.bind_pose();
        for ch in &self.channels {
            if let Some(local) = pose.locals.get_mut(ch.target_joint) {
                ch.sample_into(t, local);
            }
        }
        pose
    }

    /// Map an arbitrary `time` into the clip's valid range, either wrapping
    /// (looping) or clamping (hold last frame).
    fn resolve_time(&self, time: f32, looping: bool) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        if looping {
            // `rem_euclid` keeps the result in [0, duration) even for negative
            // times, so looping is well-behaved when rewound.
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        }
    }
}

/// An evaluated pose: one **local** transform per skeleton joint (DESIGN.md §17).
/// Feed to [`compute_joint_matrices`] to produce the GPU skinning palette.
#[derive(Clone, Debug, Default)]
pub struct Pose {
    /// Per-joint local transforms, indexed identically to [`Skeleton::joints`].
    pub locals: Vec<Transform>,
}

impl Pose {
    /// Blend `self` toward `other` by `alpha` in `[0, 1]`, per joint. `alpha = 0`
    /// yields `self`; `alpha = 1` yields `other`. Used for crossfades between two
    /// clip poses (DESIGN.md §16). Extra joints beyond the shorter pose are
    /// dropped, which never happens when both poses come from the same skeleton.
    pub fn blend(&self, other: &Pose, alpha: f32) -> Pose {
        let n = self.locals.len().min(other.locals.len());
        let mut locals = Vec::with_capacity(n);
        for i in 0..n {
            locals.push(lerp_transform(self.locals[i], other.locals[i], alpha));
        }
        Pose { locals }
    }
}

/// Per-component interpolation between two local transforms: LERP translation
/// and scale, SLERP rotation along the shortest arc (DESIGN.md §16/§17).
pub fn lerp_transform(a: Transform, b: Transform, t: f32) -> Transform {
    Transform {
        translation: a.translation.lerp(b.translation, t),
        rotation: a.rotation.slerp(b.rotation, t),
        scale: a.scale.lerp(b.scale, t),
    }
}

/// Compute the GPU skinning palette for `pose` (DESIGN.md §17 "joint matrix
/// upload").
///
/// For each joint the model-space matrix is accumulated by walking parents
/// (children inherit their ancestors), then multiplied by the joint's
/// [`Joint::inverse_bind`]. The result is `model_space_joint_matrix *
/// inverse_bind` per joint — exactly the `Vec<Mat4>` expected by
/// `pocket3d_render::SkinnedInstance::joint_matrices`.
///
/// Joints missing from `pose` fall back to their bind-local transform, so a
/// short or empty pose degrades to the bind pose rather than panicking.
pub fn compute_joint_matrices(skeleton: &Skeleton, pose: &Pose) -> Vec<Mat4> {
    let n = skeleton.joint_count();
    // model[i] = model-space (root-relative) matrix of joint i for this pose.
    let mut model = vec![Mat4::IDENTITY; n];
    let mut palette = vec![Mat4::IDENTITY; n];
    for (i, joint) in skeleton.joints.iter().enumerate() {
        let local = pose
            .locals
            .get(i)
            .copied()
            .unwrap_or(joint.local_bind)
            .matrix();
        model[i] = match joint.parent {
            // Topological ordering guarantees `p < i`, so `model[p]` is ready.
            Some(p) => model[p] * local,
            None => local,
        };
        palette[i] = model[i] * joint.inverse_bind;
    }
    palette
}

/// Bot animation states (DESIGN.md §16). `Run`/`HitReact` are listed as optional
/// in the design and are deferred; the v0 bot state machine covers idle, walk,
/// and death.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimState {
    /// Standing still.
    Idle,
    /// Locomoting along a waypoint path.
    Walk,
    /// Dead — plays once and holds the final frame.
    Death,
}

impl AnimState {
    /// Whether this state's clip loops. Locomotion states loop; `Death` holds
    /// its last frame (DESIGN.md §16).
    fn loops(self) -> bool {
        !matches!(self, AnimState::Death)
    }
}

/// Default bot crossfade duration in seconds (DESIGN.md §16: "simple crossfade,
/// e.g. 150 ms").
pub const DEFAULT_CROSSFADE_SECS: f32 = 0.150;

/// A tiny crossfading animation state machine for the v0 bot (DESIGN.md §16).
///
/// It owns one [`AnimationClip`] per [`AnimState`] and tracks the current and
/// previous state, independent playback clocks for each, and a crossfade timer.
/// [`AnimStateMachine::sample`] blends the previous pose into the current pose by
/// an alpha that ramps `0 → 1` over the crossfade window.
#[derive(Clone, Debug)]
pub struct AnimStateMachine {
    /// Clips indexed as `[Idle, Walk, Death]`.
    clips: [AnimationClip; 3],
    /// The state currently being driven toward.
    current: AnimState,
    /// The state being faded out (equals `current` when not crossfading).
    previous: AnimState,
    /// Local playback time (seconds) of the current clip.
    current_time: f32,
    /// Local playback time (seconds) of the previous clip.
    previous_time: f32,
    /// Seconds remaining in the active crossfade; `0` means no blend.
    crossfade_remaining: f32,
    /// Total crossfade window (seconds).
    crossfade_duration: f32,
}

impl AnimStateMachine {
    /// Create a state machine starting in [`AnimState::Idle`] with the default
    /// 150 ms crossfade (DESIGN.md §16). Provide one clip per state.
    pub fn new(idle: AnimationClip, walk: AnimationClip, death: AnimationClip) -> Self {
        Self {
            clips: [idle, walk, death],
            current: AnimState::Idle,
            previous: AnimState::Idle,
            current_time: 0.0,
            previous_time: 0.0,
            crossfade_remaining: 0.0,
            crossfade_duration: DEFAULT_CROSSFADE_SECS,
        }
    }

    /// Override the crossfade window (seconds). Builder-style.
    pub fn with_crossfade(mut self, secs: f32) -> Self {
        self.crossfade_duration = secs.max(0.0);
        self
    }

    /// The state currently being driven toward.
    pub fn current(&self) -> AnimState {
        self.current
    }

    /// The state being faded out (equals [`Self::current`] when settled).
    pub fn previous(&self) -> AnimState {
        self.previous
    }

    /// Whether a crossfade is currently in progress.
    pub fn is_crossfading(&self) -> bool {
        self.crossfade_remaining > 0.0 && self.previous != self.current
    }

    /// Request a transition to `state`. If it differs from the current state the
    /// current state becomes `previous`, the new clip restarts, and a fresh
    /// crossfade begins (DESIGN.md §16). Re-requesting the current state is a
    /// no-op, so this is safe to call every frame.
    pub fn request(&mut self, state: AnimState) {
        if state == self.current {
            return;
        }
        self.previous = self.current;
        self.previous_time = self.current_time;
        self.current = state;
        self.current_time = 0.0;
        self.crossfade_remaining = self.crossfade_duration;
    }

    /// Advance the playback clocks and the crossfade timer by `dt` seconds
    /// (DESIGN.md §16).
    pub fn update(&mut self, dt: f32) {
        self.current_time += dt;
        self.previous_time += dt;
        if self.crossfade_remaining > 0.0 {
            self.crossfade_remaining = (self.crossfade_remaining - dt).max(0.0);
        }
    }

    /// Look up the clip backing a state.
    fn clip(&self, state: AnimState) -> &AnimationClip {
        match state {
            AnimState::Idle => &self.clips[0],
            AnimState::Walk => &self.clips[1],
            AnimState::Death => &self.clips[2],
        }
    }

    /// Evaluate the current pose for `skeleton`, blending the previous state's
    /// pose into the current state's pose by the crossfade alpha (DESIGN.md §16).
    /// When no crossfade is active this is just the current clip's pose.
    pub fn sample(&self, skeleton: &Skeleton) -> Pose {
        let current_pose = self.clip(self.current).sample(
            skeleton,
            self.current_time,
            self.current.loops(),
        );

        if !self.is_crossfading() {
            return current_pose;
        }

        // alpha ramps 0 → 1 as the crossfade completes (previous → current).
        let alpha = 1.0 - (self.crossfade_remaining / self.crossfade_duration);
        let previous_pose = self.clip(self.previous).sample(
            skeleton,
            self.previous_time,
            self.previous.loops(),
        );
        previous_pose.blend(&current_pose, alpha)
    }
}

// ---------------------------------------------------------------------------
// Test/example builders
// ---------------------------------------------------------------------------

/// Build a trivial two-joint skeleton for tests and examples: a root at the
/// origin and a child offset by `child_offset` in the root's local space. The
/// [`Joint::inverse_bind`] matrices are computed from the bind pose so the
/// skinning palette is the identity at rest.
pub fn test_skeleton(child_offset: Vec3) -> Skeleton {
    let mut skeleton = Skeleton {
        joints: vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                local_bind: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: "child".to_string(),
                parent: Some(0),
                local_bind: Transform::from_translation(child_offset),
                inverse_bind: Mat4::IDENTITY,
            },
        ],
    };
    // inverse_bind = inverse of each joint's model-space bind matrix.
    let model = skeleton.bind_model_matrices();
    for (joint, m) in skeleton.joints.iter_mut().zip(model) {
        joint.inverse_bind = m.inverse();
    }
    skeleton
}

/// Build a two-keyframe translation clip that drives `joint`'s local translation
/// from `start` at `t = 0` to `end` at `t = duration`. Handy for exercising clip
/// sampling in tests.
pub fn test_translation_clip(
    name: &str,
    joint: usize,
    start: Vec3,
    end: Vec3,
    duration: f32,
) -> AnimationClip {
    AnimationClip {
        name: name.to_string(),
        duration,
        channels: vec![Channel {
            target_joint: joint,
            kind: ChannelKind::Translation,
            times: vec![0.0, duration],
            values: vec![start.extend(0.0), end.extend(0.0)],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (DESIGN.md §17) A translation channel on the child joint of a two-joint
    /// skeleton should leave the child put at `t = 0` (skinning matrix == I) and
    /// displace it by the interpolated amount at the midpoint.
    #[test]
    fn translation_clip_moves_child() {
        let child_offset = Vec3::new(1.0, 0.0, 0.0);
        let skeleton = test_skeleton(child_offset);

        // Drive the child's local translation from its bind offset to
        // offset + (0, 2, 0) over one second.
        let start = child_offset;
        let end = child_offset + Vec3::new(0.0, 2.0, 0.0);
        let clip = test_translation_clip("move", 1, start, end, 1.0);

        // At t = 0 the animated pose equals the bind pose, so each skinning
        // matrix is the identity and the child's bind-space point is unchanged.
        let pose0 = clip.sample(&skeleton, 0.0, false);
        let mats0 = compute_joint_matrices(&skeleton, &pose0);
        assert_eq!(mats0.len(), 2);
        let bind_point = child_offset;
        let moved0 = mats0[1].transform_point3(bind_point);
        assert!(
            (moved0 - bind_point).length() < 1e-4,
            "child should be at rest at t=0, got {moved0}"
        );

        // At the midpoint the child's local translation is start + (0, 1, 0),
        // i.e. +1 in Y relative to bind, so the skinning matrix must translate
        // the bind point by (0, 1, 0).
        let pose_mid = clip.sample(&skeleton, 0.5, false);
        let mats_mid = compute_joint_matrices(&skeleton, &pose_mid);
        let moved_mid = mats_mid[1].transform_point3(bind_point);
        let expected = bind_point + Vec3::new(0.0, 1.0, 0.0);
        assert!(
            (moved_mid - expected).length() < 1e-4,
            "child should move to {expected}, got {moved_mid}"
        );

        // The root is unanimated, so its skinning matrix stays the identity.
        assert!(mats_mid[0].abs_diff_eq(Mat4::IDENTITY, 1e-4));
    }

    /// (DESIGN.md §16) A crossfade should blend the previous pose into the new
    /// pose, reaching the halfway blend at half the crossfade window and the
    /// full new pose once the window elapses.
    #[test]
    fn crossfade_blends_poses() {
        let skeleton = test_skeleton(Vec3::X);

        // Idle: no channels, so the child keeps its bind translation of +X.
        let idle = AnimationClip {
            name: "idle".to_string(),
            duration: 1.0,
            channels: Vec::new(),
        };
        // Walk: pin the child's local translation to X + (0, 4, 0) (constant).
        let walk_translation = Vec3::X + Vec3::new(0.0, 4.0, 0.0);
        let walk =
            test_translation_clip("walk", 1, walk_translation, walk_translation, 1.0);
        let death = idle.clone();

        let mut sm = AnimStateMachine::new(idle, walk, death);
        assert_eq!(sm.current(), AnimState::Idle);
        assert!(!sm.is_crossfading());

        // Begin crossfading Idle -> Walk.
        sm.request(AnimState::Walk);
        assert!(sm.is_crossfading());

        // At alpha = 0 the blended pose equals the idle (previous) pose.
        let p_start = sm.sample(&skeleton);
        assert!(
            (p_start.locals[1].translation - Vec3::X).length() < 1e-3,
            "start of crossfade should equal idle, got {}",
            p_start.locals[1].translation
        );

        // Halfway through the 150 ms window -> alpha = 0.5 -> midpoint blend.
        sm.update(DEFAULT_CROSSFADE_SECS * 0.5);
        let p_mid = sm.sample(&skeleton);
        let expected_mid = Vec3::X + Vec3::new(0.0, 2.0, 0.0);
        assert!(
            (p_mid.locals[1].translation - expected_mid).length() < 1e-3,
            "midway crossfade should be {expected_mid}, got {}",
            p_mid.locals[1].translation
        );

        // Past the window -> crossfade settled -> pure walk pose.
        sm.update(DEFAULT_CROSSFADE_SECS);
        assert!(!sm.is_crossfading());
        let p_end = sm.sample(&skeleton);
        assert!(
            (p_end.locals[1].translation - walk_translation).length() < 1e-3,
            "end of crossfade should equal walk, got {}",
            p_end.locals[1].translation
        );
    }

    /// Looping clips wrap time by their duration; clamped clips hold the last
    /// keyframe (DESIGN.md §17).
    #[test]
    fn looping_wraps_and_clamp_holds() {
        let skeleton = test_skeleton(Vec3::ZERO);
        let clip = test_translation_clip("t", 1, Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), 2.0);

        // Looping: t = 2.5 wraps to 0.5 -> quarter of the way (0, 2.5, 0).
        let looped = clip.sample(&skeleton, 2.5, true);
        assert!(
            (looped.locals[1].translation - Vec3::new(0.0, 2.5, 0.0)).length() < 1e-4,
            "got {}",
            looped.locals[1].translation
        );

        // Clamped: t beyond duration holds the final keyframe (0, 10, 0).
        let clamped = clip.sample(&skeleton, 5.0, false);
        assert!(
            (clamped.locals[1].translation - Vec3::new(0.0, 10.0, 0.0)).length() < 1e-4,
            "got {}",
            clamped.locals[1].translation
        );
    }

    /// The bind-pose skinning palette must be all-identity (DESIGN.md §17).
    #[test]
    fn bind_pose_palette_is_identity() {
        let skeleton = test_skeleton(Vec3::new(0.0, 3.0, 1.0));
        let palette = compute_joint_matrices(&skeleton, &skeleton.bind_pose());
        for m in palette {
            assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-4));
        }
    }
}
