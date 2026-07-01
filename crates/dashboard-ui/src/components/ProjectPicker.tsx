import { useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export type ProjectPickerOption = {
  id: string;
  name: string;
  root_path?: string;
};

function pathLabel(path?: string): string {
  if (!path) return "";
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts.length > 2 ? `…/${parts.slice(-2).join("/")}` : path;
}

export function ProjectPicker({
  value,
  options,
  onChange,
  disabled = false,
  includeAll = false,
  className = "",
  buttonClassName = "",
}: {
  value: string;
  options: ProjectPickerOption[];
  onChange: (projectId: string) => void;
  disabled?: boolean;
  includeAll?: boolean;
  className?: string;
  buttonClassName?: string;
}) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  useEffect(() => {
    if (!open) setQuery("");
  }, [open]);

  const selected = options.find((p) => p.id === value);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return options;
    return options.filter((p) =>
      [p.name, p.root_path].filter(Boolean).join(" ").toLowerCase().includes(q),
    );
  }, [options, query]);

  const label = value
    ? selected?.name || t("projectPicker.unknownProject")
    : includeAll
      ? t("conversations.allProjects")
      : t("home.hero.noProject");

  function selectProject(projectId: string) {
    onChange(projectId);
    setOpen(false);
  }

  return (
    <div className={`project-picker ${className}`} ref={ref}>
      <button
        type="button"
        className={`project-picker__button ${buttonClassName}`}
        disabled={disabled}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <Icon name="folder" size={16} className="text-secondary shrink-0" />
        <span className="project-picker__button-text">{label}</span>
        <Icon name={open ? "expand_less" : "expand_more"} size={14} className="text-secondary shrink-0" />
      </button>

      {open && (
        <div className="project-picker__popover" role="dialog" aria-label={t("projectPicker.title")}>
          <div className="project-picker__search">
            <Icon name="search" size={16} className="text-outline shrink-0" />
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("projectPicker.placeholder")}
            />
          </div>
          <div className="project-picker__section-label">{t("projectPicker.recents")}</div>
          <div className="project-picker__list">
            {includeAll && (
              <button
                type="button"
                className={`project-picker__item${value === "" ? " active" : ""}`}
                onClick={() => selectProject("")}
              >
                <Icon name="dashboard" size={17} className="text-secondary shrink-0" />
                <span className="project-picker__item-main">
                  <span className="project-picker__item-title">{t("conversations.allProjects")}</span>
                </span>
                {value === "" && <Icon name="check" size={16} className="text-primary shrink-0" />}
              </button>
            )}
            {filtered.map((project) => (
              <button
                type="button"
                key={project.id}
                className={`project-picker__item${project.id === value ? " active" : ""}`}
                onClick={() => selectProject(project.id)}
              >
                <Icon name="folder" size={17} className="text-secondary shrink-0" />
                <span className="project-picker__item-main">
                  <span className="project-picker__item-title">{project.name}</span>
                  {project.root_path && (
                    <span className="project-picker__item-path">{pathLabel(project.root_path)}</span>
                  )}
                </span>
                {project.id === value && <Icon name="check" size={16} className="text-primary shrink-0" />}
              </button>
            ))}
            {filtered.length === 0 && (
              <div className="project-picker__empty">{t("projectPicker.empty")}</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
