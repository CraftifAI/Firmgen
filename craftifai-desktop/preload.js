'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('craftifai', {
  // Settings
  getSettings:         ()       => ipcRenderer.invoke('get-settings'),
  saveWizardSettings:  (data)   => ipcRenderer.invoke('save-wizard-settings', data),
  resetSettings:       ()       => ipcRenderer.invoke('reset-settings'),

  // File dialogs
  browseFolder:   ()       => ipcRenderer.invoke('browse-folder'),
  browseFile:     (opts)   => ipcRenderer.invoke('browse-file', opts),

  // Logs
  getLogDir:      ()       => ipcRenderer.invoke('get-log-dir'),
  openLogs:       ()       => ipcRenderer.invoke('open-logs'),

  // Status updates from main → renderer
  onStatus:       (cb)     => ipcRenderer.on('status', (_e, msg) => cb(msg)),
  onError:        (cb)     => ipcRenderer.on('error-message', (_e, msg) => cb(msg)),
});
