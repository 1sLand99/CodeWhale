import * as vscode from "vscode";

const SECRET_KEY = "codewhale.runtimeToken";

/**
 * Resolve the runtime bearer token: SecretStorage first, then the legacy
 * `codewhale.runtimeToken` setting. The setting is kept for compatibility
 * but SecretStorage is where new tokens go, so tokens stop living in
 * plaintext settings.json / workspace dotfiles.
 */
export async function resolveToken(context: vscode.ExtensionContext): Promise<string | undefined> {
  const stored = await context.secrets.get(SECRET_KEY);
  if (stored && stored.trim().length > 0) {
    return stored.trim();
  }
  const setting = vscode.workspace
    .getConfiguration("codewhale")
    .get<string>("runtimeToken", "")
    .trim();
  if (setting) {
    // Migrate a settings-based token into secret storage so it can be
    // removed from the (possibly synced) settings file.
    await context.secrets.store(SECRET_KEY, setting);
    return setting;
  }
  return undefined;
}

export async function storeToken(context: vscode.ExtensionContext, token: string): Promise<void> {
  await context.secrets.store(SECRET_KEY, token.trim());
}

export async function promptForToken(context: vscode.ExtensionContext): Promise<string | undefined> {
  const entered = await vscode.window.showInputBox({
    prompt: "Codewhale runtime bearer token (stored in VS Code secret storage)",
    password: true,
    ignoreFocusOut: true,
  });
  if (entered === undefined) {
    return undefined;
  }
  const token = entered.trim();
  if (token.length === 0) {
    await context.secrets.delete(SECRET_KEY);
    return undefined;
  }
  await storeToken(context, token);
  return token;
}
