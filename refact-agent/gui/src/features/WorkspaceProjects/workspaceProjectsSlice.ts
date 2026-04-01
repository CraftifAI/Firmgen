import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { v4 as uuidv4 } from "uuid";

export type WorkspaceProject = {
  id: string;
  name: string;
  esp32_projects_path: string;
  createdAt: string;
};

export type WorkspaceProjectsState = {
  projects: WorkspaceProject[];
  activeProjectId: string | null;
  projectsSectionExpanded: boolean;
};

const initialState: WorkspaceProjectsState = {
  projects: [],
  activeProjectId: null,
  projectsSectionExpanded: true,
};

export const workspaceProjectsSlice = createSlice({
  name: "workspaceProjects",
  initialState,
  reducers: {
    addProject: (
      state,
      action: PayloadAction<{
        name: string;
        esp32_projects_path: string;
        id?: string;
        createdAt?: string;
      }>,
    ) => {
      const id = action.payload.id ?? uuidv4();
      const createdAt = action.payload.createdAt ?? new Date().toISOString();
      state.projects.push({
        id,
        name: action.payload.name,
        esp32_projects_path: action.payload.esp32_projects_path,
        createdAt,
      });
      state.activeProjectId = id;
    },
    removeProject: (state, action: PayloadAction<string>) => {
      state.projects = state.projects.filter((p) => p.id !== action.payload);
      if (state.activeProjectId === action.payload) {
        state.activeProjectId = null;
      }
    },
    setActiveProjectId: (state, action: PayloadAction<string | null>) => {
      state.activeProjectId = action.payload;
    },
    toggleProjectsSection: (state) => {
      state.projectsSectionExpanded = !state.projectsSectionExpanded;
    },
    setProjectsSectionExpanded: (state, action: PayloadAction<boolean>) => {
      state.projectsSectionExpanded = action.payload;
    },
    renameProject: (
      state,
      action: PayloadAction<{ id: string; name: string }>,
    ) => {
      const p = state.projects.find((x) => x.id === action.payload.id);
      if (p) p.name = action.payload.name;
    },
  },
});

export const {
  addProject,
  removeProject,
  setActiveProjectId,
  toggleProjectsSection,
  setProjectsSectionExpanded,
  renameProject,
} = workspaceProjectsSlice.actions;
