import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  server: {
    port: 1420,
    strictPort: true,
    // Cargo replaces executables here; watching them can fail with EBUSY on Windows.
    watch: { ignored: ["**/target/**"] },
  },
  clearScreen: false,
});
