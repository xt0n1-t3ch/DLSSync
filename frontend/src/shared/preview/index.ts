export interface PreviewFixture<T> {
  id: string;
  value: T;
}

export function previewFixture<T>(fixture: PreviewFixture<T>): T {
  if (!import.meta.env.DEV) {
    throw new Error("Preview fixtures are unavailable in release builds");
  }
  return structuredClone(fixture.value);
}
