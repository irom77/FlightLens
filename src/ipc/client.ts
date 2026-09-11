import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Artifact,
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
export const desktopAvailable = isTauri;
export const api = {
  ingest: (text: string, label: string) =>
    invoke<Artifact>("ingest_text", { text, label }),
  chooseFiles: () => invoke<SourceDescriptor[]>("choose_files"),
  open: (sourceId: string) => invoke<Artifact>("open_source", { sourceId }),
  pending: () => invoke<SourceDescriptor[]>("pending_sources"),
  restore: () => invoke<RestoredSession>("restore_session"),
  close: (configId: string) => invoke<void>("close_document", { configId }),
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
