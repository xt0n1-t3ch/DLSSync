import eslintPluginSvelte from "eslint-plugin-svelte";
import globals from "globals";
import tseslint from "typescript-eslint";

export default [
  { ignores: ["dist/**", "src/generated/**", "coverage/**"] },
  ...tseslint.configs.recommended,
  ...eslintPluginSvelte.configs["flat/recommended"],
  {
    files: ["src/**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".svelte"],
      },
    },
  },
  {
    files: ["src/**/*.{ts,svelte}"],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      "no-debugger": "error",
      "svelte/prefer-svelte-reactivity": "off",
      "svelte/prefer-writable-derived": "off",
      "svelte/require-each-key": "off",
    },
  },
  {
    files: ["src/features/**/*.{ts,svelte}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["../../features/*/**"],
              message: "Import another feature only through its public index.",
            },
          ],
        },
      ],
    },
  },
];
