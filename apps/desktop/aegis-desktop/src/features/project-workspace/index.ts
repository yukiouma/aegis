// Public API of the project-workspace feature.
export { useProjectConfiguration } from "./data/project-configuration";
export {
  useActiveBranchId,
  useCreateProjectBranch,
  useProjectBranches,
  useSetActiveBranchId,
  useUpdateProjectBranch,
} from "./data/project-branches";
export { CrfStatusPieCard } from "./components/CrfStatusPieCard";
export { ConfigurationFilepathSection } from "./components/ConfigurationFilepathSection";
export { ConfigurationGeneralSection } from "./components/ConfigurationGeneralSection";
export { ConfigurationMembersSection } from "./components/ConfigurationMembersSection";
export {
  ConfigurationSidebar,
  type ConfigurationSection,
} from "./components/ConfigurationSidebar";
export { BranchNameDialog } from "./components/BranchNameDialog";
export { ProjectBranchesSection } from "./components/ProjectBranchesSection";

// The scaffold feature owns the dialog and its hook; re-exported here
// so consumers of the project-workspace barrel keep working.
export {
  InitializeProjectFolderDialog,
  useInitializeProjectFolder,
} from "../scaffold";