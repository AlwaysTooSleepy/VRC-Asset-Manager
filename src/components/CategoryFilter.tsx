interface Category { id: string; name: string }

interface Props {
  categories: Category[];
  value: string | null; // null = all categories
  onChange: (id: string | null) => void;
}

/** Dropdown filtering a compatible-asset grid by category. */
export default function CategoryFilter({ categories, value, onChange }: Props) {
  if (categories.length === 0) return null;
  return (
    <select
      className="input select-native category-filter"
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || null)}
    >
      <option value="">All categories</option>
      {[...categories]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((c) => (
          <option key={c.id} value={c.id}>{c.name}</option>
        ))}
    </select>
  );
}
