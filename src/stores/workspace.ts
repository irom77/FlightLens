import { create } from "zustand";
export const tabs = [
  "Rates",
  "PID",
  "Filters",
  "Ports",
  "Modes",
  "OSD",
  "Raw",
  "Audit",
  "Export",
] as const;
export type Tab = (typeof tabs)[number];
interface DocumentSummary {
  id: string;
  title: string;
  family: string;
}
interface WorkspaceState {
  documents: DocumentSummary[];
  activeId: string | null;
  tab: Tab;
  add: (document: DocumentSummary) => void;
  close: (id: string) => void;
  activate: (id: string) => void;
  setTab: (tab: Tab) => void;
}
export const useWorkspace = create<WorkspaceState>((set) => ({
  documents: [],
  activeId: null,
  tab: "Rates",
  add: (document) =>
    set((s) => ({
      documents: [...s.documents.filter((d) => d.id !== document.id), document],
      activeId: document.id,
    })),
  close: (id) =>
    set((s) => ({
      documents: s.documents.filter((d) => d.id !== id),
      activeId:
        s.activeId === id
          ? (s.documents.find((d) => d.id !== id)?.id ?? null)
          : s.activeId,
    })),
  activate: (activeId) => set({ activeId }),
  setTab: (tab) => {
    try {
      localStorage.setItem("flightlens.tab", tab);
    } catch {
      /* UI preferences are optional. */
    }
    set({ tab });
  },
}));
