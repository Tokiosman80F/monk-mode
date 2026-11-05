import appConfig from "../app/tailwind.config";

// Same themes and motion as the app. Only the files to scan differ.
export default {
  ...appConfig,
  content: ["./block.html", "./src/block.ts"],
};
