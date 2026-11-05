import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["**/dist/", "**/target/", "app/src-tauri/gen/", "**/src/types/"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["app/src/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: reactHooks.configs.recommended.rules,
  },
);
