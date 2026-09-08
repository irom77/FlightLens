import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { Plot } from "./Plots";
it("keeps unknown curves absent and displays their reason", () => {
  const html = renderToStaticMarkup(
    <Plot
      curves={[
        {
          name: "roll",
          points: [],
          derivedInputs: [],
          reason: "roll_expo is unknown",
        },
      ]}
    />,
  );
  expect(html).toContain("roll_expo is unknown");
  expect(html).not.toContain("<path");
});
it("renders backend samples and escapes untrusted labels", () => {
  const html = renderToStaticMarkup(
    <Plot
      curves={[
        {
          name: "<script>bad</script>",
          points: [
            { x: -1, y: -800 },
            { x: 0, y: 0 },
            { x: 1, y: 800 },
          ],
          derivedInputs: [],
          reason: null,
        },
      ]}
    />,
  );
  expect(html).toContain("<path");
  expect(html).not.toContain("<script>");
  expect(html).toContain("&lt;script&gt;");
});
it("preserves the second source style when the first curve is unavailable", () => {
  const html = renderToStaticMarkup(
    <Plot
      curves={[
        { name: "A", points: [], derivedInputs: [], reason: "Unknown" },
        {
          name: "B",
          points: [{ x: 0, y: 0 }],
          derivedInputs: [],
          reason: null,
        },
      ]}
    />,
  );
  expect(html).toContain('class="series-1"');
  expect(html).toContain('stroke-dasharray="7 3"');
});
