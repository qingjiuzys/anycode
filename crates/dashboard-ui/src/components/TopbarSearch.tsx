import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SearchResultsDropdown } from "@/components/SearchResultsDropdown";
import { useT } from "@/i18n/context";

/** Compact global search for the top bar (prototype parity). */
export function TopbarSearch() {
  const t = useT();
  const [q, setQ] = useState("");
  const [open, setOpen] = useState(false);
  const { data } = useQuery({
    queryKey: ["search", q],
    queryFn: () => api.search(q, 8),
    enabled: q.trim().length >= 2,
  });

  return (
    <div className="relative hidden md:block w-full max-w-[14rem] lg:max-w-[16rem] min-w-0">
      <Icon
        name="search"
        size={18}
        className="absolute left-2.5 top-1/2 -translate-y-1/2 text-on-surface-variant"
      />
      <input
        type="search"
        className="w-full bg-surface-container-low border-0 rounded-lg py-2 pl-9 pr-3 text-sm text-on-surface placeholder:text-on-surface-variant focus:ring-1 focus:ring-primary outline-none"
        placeholder={t("search.placeholder")}
        value={q}
        onChange={(e) => {
          setQ(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
      />
      {open && q.trim().length >= 2 && data && (
        <div className="absolute top-full left-0 right-0 mt-1 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg z-50 max-h-64 overflow-y-auto">
          <SearchResultsDropdown data={data} />
        </div>
      )}
    </div>
  );
}
