//! Shared material description.
//!
//! `MaterialKind` is defined here (rather than in `pocket3d-render`) so the BSP
//! importer can classify surfaces without depending on the renderer. The
//! renderer re-exports it.

/// Which shading path a surface uses (DESIGN.md §12).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialKind {
    /// BSP world surface: `base_texture * lightmap`.
    BspWorldLit,
    /// Sky surface (rendered without lightmap; drawn far).
    BspSky,
    /// Translucent water surface.
    BspWater,
    /// Alpha-tested BSP surface (textures prefixed `{` in GoldSrc).
    BspAlphaTest,
    /// Static prop, no lighting.
    StaticUnlit,
    /// Static prop, simple lighting.
    StaticLit,
    /// Skinned character, simple lighting.
    SkinnedLit,
    /// First-person weapon viewmodel.
    Viewmodel,
    /// Debug lines/points.
    Debug,
}

impl MaterialKind {
    pub fn is_translucent(self) -> bool {
        matches!(self, MaterialKind::BspWater)
    }

    pub fn is_alpha_tested(self) -> bool {
        matches!(self, MaterialKind::BspAlphaTest)
    }
}

/// A material description produced by importers and consumed by the renderer.
#[derive(Clone, Debug)]
pub struct MaterialDesc {
    pub name: String,
    pub kind: MaterialKind,
    /// Index into the owning asset's texture table, if any.
    pub base_texture: Option<u32>,
    /// Which lightmap atlas page this material samples, if any.
    pub lightmap_atlas: Option<u32>,
}

impl MaterialDesc {
    pub fn new(name: impl Into<String>, kind: MaterialKind) -> Self {
        Self {
            name: name.into(),
            kind,
            base_texture: None,
            lightmap_atlas: None,
        }
    }
}
