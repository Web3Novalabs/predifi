interface SearchResultHighlighterProps {
  text: string;
  searchQuery: string;
}

/**
 * Splits `text` on case-insensitive matches of `searchQuery` and wraps each
 * match in a styled `<mark>`. Returns the plain text node when the query is
 * empty or produces no match.
 */
export function SearchResultHighlighter({
  text,
  searchQuery,
}: SearchResultHighlighterProps) {
  const query = searchQuery.trim();

  if (!query) return <>{text}</>;

  // Escape regex metacharacters so user input is treated as literal text.
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  const parts = text.split(regex);

  if (parts.length === 1) return <>{text}</>;

  return (
    <>
      {parts.map((part, i) =>
        regex.test(part) ? (
          <mark
            key={i}
            className="bg-primary/20 text-primary font-medium rounded-sm px-0.5"
          >
            {part}
          </mark>
        ) : (
          <span key={i}>{part}</span>
        )
      )}
    </>
  );
}
