import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SearchResultsDropdown } from "@/components/SearchResultsDropdown";
import { useT } from "@/i18n/context";

export function SearchBox() {
  const t = useT();
  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  const { data } = useQuery({
    queryKey: ["search", q],
    queryFn: () => api.search(q, 8),
    enabled: q.trim().length >= 2,
  });

  return (
    <div className="relative">
      <div className="relative">
        <Icon
          name="search"
          size={16}
          className="absolute left-2.5 top-1/2 -translate-y-1/2 text-outline"
        />
        <input
          type="search"
          className="dw-input w-full pl-8 text-xs py-2"
          placeholder={t("layout.search")}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 150)}
        />
      </div>
      {open && q.trim().length >= 2 && data && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-surface-container-lowest border border-outline-variant rounded shadow-lg z-50 max-h-64 overflow-y-auto">
          <SearchResultsDropdown data={data} />
        </div>
      )}
    </div>
  );
}
