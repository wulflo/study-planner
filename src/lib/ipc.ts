import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  schemaVersion: number;
  exportFormatVersion: number;
}

export interface SchemaCheck {
  schemaVersion: number;
  integrity: string;
  foreignKeyViolations: number;
  vaultId: string;
}

export const runningInTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

export async function runSchemaSelfCheck(): Promise<SchemaCheck> {
  return invoke<SchemaCheck>("schema_self_check");
}
