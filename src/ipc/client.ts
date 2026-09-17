import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  LlmSettings,
  LlmStatus,
  LlmPreview,
  LlmSummary,
  SummaryRequest,
  Provider,
  ArtifactView,
  SyntaxLine,
  WorkspacePage,
  ExportRequest,
  FeedbackReport,
  FeedbackRequest,
  Inspection,
  Point,
  RestoredSession,
  Scope,
  SourceDescriptor,
  ValidatedSnippet,
} from "../bindings/core";
type SessionProfiles = {
  documentId: string;
  rateProfile: number;
  pidProfile: number;
};
type SessionComparison = { documents: string[]; baseline: string | null };
export const desktopAvailable = isTauri;
export const api = {
  llmSettings: () => invoke<LlmStatus>("llm_settings"),
  llmSaveSettings: (settings: LlmSettings) =>
    invoke<LlmStatus>("llm_save_settings", { settings }),
  llmSaveKey: (provider: Provider, key: string, sessionOnly = false) =>
    invoke<LlmStatus>("llm_save_key", { provider, key, sessionOnly }),
  llmClearKey: (provider: Provider) =>
    invoke<LlmStatus>("llm_clear_key", { provider }),
  llmPreview: (request: SummaryRequest) =>
    invoke<LlmPreview>("llm_preview", { request }),
  llmSummarize: (request: SummaryRequest, regenerate = false) =>
    invoke<LlmSummary>("llm_summarize", { request, regenerate }),
  llmSaveSummary: (request: SummaryRequest, summary: LlmSummary) =>
    invoke<boolean>("llm_save_summary", { request, summary }),
  llmCancel: () => invoke<void>("llm_cancel"),
  llmTestPreview: () => invoke<LlmPreview>("llm_test_preview"),
  llmTestConnection: () => invoke<string>("llm_test_connection"),
  savePortableSession: (request: {
    preserveUnavailableSelections: boolean;
    documentIds: string[];
    profiles: SessionProfiles[];
    comparison: SessionComparison | null;
    activeId: string | null;
    tab: string;
    theme: string;
  }) => invoke<boolean>("save_portable_session", { request }),
  openPortableSession: (retry = false, relink = false) =>
    invoke<{
      workspace: WorkspacePage | null;
      artifacts: ArtifactView[];
      profiles: SessionProfiles[];
      comparison: SessionComparison | null;
      activeId: string | null;
      tab: string;
      theme: string;
      warnings: string[];
    } | null>("open_portable_session", { retry, relink }),
  chooseWorkspace: (refresh = false) =>
    invoke<WorkspacePage | null>("choose_workspace", { refresh }),
  workspacePage: (query: string, offset: number) =>
    invoke<WorkspacePage | null>("workspace_page", { query, offset }),
  cancelWorkspace: () => invoke<void>("cancel_workspace"),
  openWorkspaceEntry: (entryId: string) =>
    invoke<ArtifactView>("open_workspace_entry", { entryId }),
  ingest: (text: string, label: string) =>
    invoke<ArtifactView>("ingest_text", { text, label }),
  chooseFiles: () => invoke<SourceDescriptor[]>("choose_files"),
  open: (sourceId: string) => invoke<ArtifactView>("open_source", { sourceId }),
  pending: () => invoke<SourceDescriptor[]>("pending_sources"),
  restore: () => invoke<RestoredSession>("restore_session"),
  close: (configId: string) => invoke<void>("close_document", { configId }),
  rawPage: (id: string, offset: number) =>
    invoke<SyntaxLine[]>("raw_page", { id, offset, count: 500 }),
  inspect: (configId: string, rateProfile: number) =>
    invoke<Inspection>("inspect_config", { configId, rateProfile }),
  filter: (configId: string, key: string, scope: Scope, sampleRate: number) =>
    invoke<Point[]>("filter_plot", { configId, key, scope, sampleRate }),
  export: (configId: string, request: ExportRequest) =>
    invoke<ValidatedSnippet>("export_snippet", { configId, request }),
  save: (configId: string, request: ExportRequest) =>
    invoke<boolean>("save_snippet", { configId, request }),
  // The report is assembled in Rust so the dialog can show exactly what the
  // browser and the clipboard will receive, and filing rebuilds it there from
  // the same fields rather than opening a URL the page supplies.
  feedbackReport: (configId: string | null, request: FeedbackRequest) =>
    invoke<FeedbackReport>("feedback_report", { configId, request }),
  fileFeedback: (configId: string | null, request: FeedbackRequest) =>
    invoke<void>("file_feedback_report", { configId, request }),
};
