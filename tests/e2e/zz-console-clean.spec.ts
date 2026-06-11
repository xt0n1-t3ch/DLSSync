import { test, expect } from "./fixtures";

test.describe("console hygiene", () => {
  test("no console errors and no uncaught exceptions across the run", async ({ app }, testInfo) => {
    const { errors, exceptions, ignored } = app.noise;
    if (ignored.length > 0) {
      testInfo.annotations.push({
        type: "ignored-noise",
        description: `${ignored.length} benign external-resource console messages: ${JSON.stringify(ignored.slice(0, 5))}`,
      });
    }
    expect(errors, `console errors: ${JSON.stringify(errors.slice(0, 10))}`).toHaveLength(0);
    expect(exceptions, `exceptions: ${JSON.stringify(exceptions.slice(0, 10))}`).toHaveLength(0);
  });
});
