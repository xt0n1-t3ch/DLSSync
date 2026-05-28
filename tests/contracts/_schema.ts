import { expect } from "vitest";

type JsonSchema = {
  type?: string | string[];
  required?: string[];
  enum?: unknown[];
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
};

function typeMatches(value: unknown, type: string): boolean {
  switch (type) {
    case "null":
      return value === null;
    case "array":
      return Array.isArray(value);
    case "integer":
      return typeof value === "number" && Number.isInteger(value);
    case "object":
      return typeof value === "object" && value !== null && !Array.isArray(value);
    default:
      return typeof value === type;
  }
}

export function assertConforms(value: unknown, schema: JsonSchema, path = "$"): void {
  if (schema.type) {
    const types = Array.isArray(schema.type) ? schema.type : [schema.type];
    expect(types.some((t) => typeMatches(value, t)), `${path}: expected ${types.join("|")}`).toBe(true);
  }
  if (value === null) return;
  if (schema.enum) {
    expect(schema.enum, `${path}: not in enum`).toContain(value);
  }
  if (Array.isArray(value) && schema.items) {
    value.forEach((item, i) => assertConforms(item, schema.items as JsonSchema, `${path}[${i}]`));
  }
  if (typeof value === "object" && !Array.isArray(value)) {
    const obj = value as Record<string, unknown>;
    for (const key of schema.required ?? []) {
      expect(Object.prototype.hasOwnProperty.call(obj, key), `${path}.${key}: required key missing`).toBe(true);
    }
    for (const [key, sub] of Object.entries(schema.properties ?? {})) {
      if (Object.prototype.hasOwnProperty.call(obj, key)) {
        assertConforms(obj[key], sub, `${path}.${key}`);
      }
    }
  }
}
