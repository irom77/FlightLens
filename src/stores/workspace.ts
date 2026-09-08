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
const activeKey = "flightlens.active";
// Which document was in front, not what it contained: the backups themselves
// are reopened from disk by the backend, which owns the saved session.
const rememberActive = (id: string | null) => {
  try {
    if (id === null) localStorage.removeItem(activeKey);
    else localStorage.setItem(activeKey, id);
  } catch {
    /* UI preferences are optional. */
  }
};
export const savedActive = (): string | null => {
  try {
    return localStorage.getItem(activeKey);
  } catch {
    /* UI preferences are optional. */
    return null;
  }
};
interface WorkspaceState {
  documents: DocumentSummary[];
  activeId: string | null;
  tab: Tab;
  add: (document: DocumentSummary) => void;
  close: (id: string) => void;
  activate: (id: string) => void;
  setTab: (tab: Tab) => void;
}
export const useWorkspace = create<WorkspaceState>((set, get) => ({
  documents: [],
  activeId: null,
  tab: "Rates",
  add: (document) => {
    rememberActive(document.id);
    set((s) => ({
      documents: [...s.documents.filter((d) => d.id !== document.id), document],
      activeId: document.id,
    }));
  },
  close: (id) => {
    set((s) => ({
      documents: s.documents.filter((d) => d.id !== id),
      activeId:
        s.activeId === id
          ? (s.documents.find((d) => d.id !== id)?.id ?? null)
          : s.activeId,
    }));
    rememberActive(get().activeId);
  },
  activate: (activeId) => {
    rememberActive(activeId);
    set({ activeId });
  },
  setTab: (tab) => {
    try {
      localStorage.setItem("flightlens.tab", tab);
    } catch {
      /* UI preferences are optional. */
    }
    set({ tab });
  },
}));
