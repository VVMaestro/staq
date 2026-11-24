import { describe, expect, test } from "bun:test";
import { parseDescriptionSegments } from "../src/taskDescription";

describe("parseDescriptionSegments", () => {
  test("returns empty array for empty text", () => {
    expect(parseDescriptionSegments("")).toEqual([]);
  });

  test("preserves plain text and line breaks without links", () => {
    const text = "Первая строка\n\nВторая строка с текстом";
    expect(parseDescriptionSegments(text)).toEqual([{ type: "text", text }]);
  });

  test("parses single http and https links surrounded by text", () => {
    expect(
      parseDescriptionSegments("Документация: https://example.com/docs?page=2#intro и http://localhost:3000/api"),
    ).toEqual([
      { type: "text", text: "Документация: " },
      {
        type: "link",
        text: "https://example.com/docs?page=2#intro",
        href: "https://example.com/docs?page=2#intro",
      },
      { type: "text", text: " и " },
      {
        type: "link",
        text: "http://localhost:3000/api",
        href: "http://localhost:3000/api",
      },
    ]);
  });

  test("handles multiline text with multiple links", () => {
    expect(
      parseDescriptionSegments("Первая: https://a.example\nВторая: http://b.example/path\nКонец"),
    ).toEqual([
      { type: "text", text: "Первая: " },
      { type: "link", text: "https://a.example", href: "https://a.example" },
      { type: "text", text: "\nВторая: " },
      { type: "link", text: "http://b.example/path", href: "http://b.example/path" },
      { type: "text", text: "\nКонец" },
    ]);
  });

  test("trims trailing punctuation and unmatched closing brackets while keeping balanced ones", () => {
    expect(
      parseDescriptionSegments(
        "См. https://example.com/a, (https://en.wikipedia.org/wiki/Rust_(programming_language)). [https://b.test/x]!",
      ),
    ).toEqual([
      { type: "text", text: "См. " },
      { type: "link", text: "https://example.com/a", href: "https://example.com/a" },
      { type: "text", text: ", (" },
      {
        type: "link",
        text: "https://en.wikipedia.org/wiki/Rust_(programming_language)",
        href: "https://en.wikipedia.org/wiki/Rust_(programming_language)",
      },
      { type: "text", text: "). [" },
      { type: "link", text: "https://b.test/x", href: "https://b.test/x" },
      { type: "text", text: "]!" },
    ]);
  });

  test("ignores non-http(s) schemes and embedded or incomplete schemes", () => {
    const text =
      "Не ссылки: javascript:alert(1), file:///etc/passwd, mailto:user@example.com, ftp://files.example.com, http://, https://, https://., foohttps://example.com, javascript:https://evil.test, file:///https://example.com, javascript://https://evil.test";
    expect(parseDescriptionSegments(text)).toEqual([{ type: "text", text }]);
  });
});
