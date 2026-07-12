/** Stable build-time API for Pocket manifests and resolved target plans. */
export {
  POCKET_MANIFEST_SCHEMA_ID,
  POCKET_MANIFEST_VERSION,
  pocketManifestV2Schema,
  type CapabilityRequirement,
  type PackageMetadata,
  type PocketManifestV2,
} from "../../spec/pocket-manifest.ts";
export {
  POCKET_CAPABILITIES,
  POCKET_PLATFORM_CONTRACTS,
  POCKET_TARGETS,
  type PocketCapabilityId,
  type PocketTargetId,
  type PresentationMode,
  type ProvidedCapability,
  type TargetProfile,
  type Viewport,
} from "../../spec/platforms.ts";
export {
  canonicalJson,
  finalizeBuildPlan,
  hashBuildPlanContent,
  verifyBuildPlanHash,
  type RationalScale,
  type ResolvedBuildPlan,
  type ResolvedBuildPlanContent,
  type ResolvedCapability,
  type ResolvedEnhancement,
} from "./plan.ts";
export {
  resolveBuildPlan,
  validateAndResolveBuildPlan,
  validatePlatformContractRegistry,
  type ResolutionResult,
  type ResolveBuildRequest,
} from "./resolve.ts";
export {
  validatePocketManifest,
  type ContractDiagnostic,
  type ValidationResult,
} from "./validate.ts";
