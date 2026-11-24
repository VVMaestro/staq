export const MAX_TASK_DESCRIPTION_LENGTH = 2000;

export type DescriptionSegment =
  | { type: "text"; text: string }
  | { type: "link"; text: string; href: string };

const URL_PATTERN = /https?:\/\/[^\s<>]+/gi;
const INVALID_PREFIX_CHAR = /[\p{L}\p{N}_:/@]/u;
const TRAILING_PUNCTUATION = /[.,;:!?…'"»”’]$/;

const MATCHING_OPENERS: Record<string, string> = {
  ")": "(",
  "]": "[",
  "}": "{",
};

function countChar(value: string, target: string): number {
  let count = 0;
  for (const ch of value) {
    if (ch === target) count += 1;
  }
  return count;
}

function stripTrailingPunctuation(raw: string): string {
  let candidate = raw;
  while (candidate.length > 0) {
    if (TRAILING_PUNCTUATION.test(candidate)) {
      candidate = candidate.slice(0, -1);
      continue;
    }

    const lastChar = candidate[candidate.length - 1];
    const opener = MATCHING_OPENERS[lastChar];
    if (opener && countChar(candidate, lastChar) > countChar(candidate, opener)) {
      candidate = candidate.slice(0, -1);
      continue;
    }

    break;
  }
  return candidate;
}

function isValidHttpUrl(candidate: string): boolean {
  try {
    const parsed = new URL(candidate);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
    return parsed.hostname.length > 0 && !/^\.+$/.test(parsed.hostname);
  } catch {
    return false;
  }
}

export function parseDescriptionSegments(text: string): DescriptionSegment[] {
  if (!text) return [];

  const segments: DescriptionSegment[] = [];
  let cursor = 0;

  for (const match of text.matchAll(URL_PATTERN)) {
    const start = match.index ?? 0;
    if (start > 0 && INVALID_PREFIX_CHAR.test(text[start - 1])) {
      continue;
    }

    const candidate = stripTrailingPunctuation(match[0]);
    if (!isValidHttpUrl(candidate)) {
      continue;
    }

    if (start > cursor) {
      segments.push({ type: "text", text: text.slice(cursor, start) });
    }

    segments.push({ type: "link", text: candidate, href: candidate });
    cursor = start + candidate.length;
  }

  if (cursor < text.length) {
    segments.push({ type: "text", text: text.slice(cursor) });
  }

  return segments;
}
