type DiagramCopy = {
  localCore: string;
  optionalCloud: string;
  localBoundary: string;
  cloudBoundary: string;
  modelRouter: string;
  nativeMedia: string;
  secureContext: string;
};

const LOCAL_NODES = [
  { id: "01", label: "Agent Runtime" },
  { id: "02", label: "Tools + Skills" },
] as const;

export function ProductArchDiagram({ copy }: { copy: DiagramCopy }) {
  const nodes = [
    ...LOCAL_NODES.map((n, i) => ({ ...n, col: i })),
    { id: "03", label: copy.nativeMedia, col: 0 },
    { id: "04", label: copy.secureContext, col: 1 },
  ];

  const cloudServices = ["Cloud Auto", "Agnes Chat", "API"] as const;

  return (
    <figure className="nx-product-diagram">
      <svg
        className="nx-product-diagram__svg"
        viewBox="0 0 480 548"
        role="img"
        aria-labelledby="nx-product-diagram-title"
      >
        <title id="nx-product-diagram-title">{copy.modelRouter}</title>

        <defs>
          <linearGradient id="nx-diagram-bg" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#ffffff" />
            <stop offset="48%" stopColor="#f8f9fb" />
            <stop offset="100%" stopColor="#eef1f4" />
          </linearGradient>
          <linearGradient id="nx-local-panel" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#ffffff" />
            <stop offset="100%" stopColor="#f6f7f9" />
          </linearGradient>
          <linearGradient id="nx-cloud-panel" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#fcfcfd" />
            <stop offset="100%" stopColor="#f2f4f7" />
          </linearGradient>
          <linearGradient id="nx-bridge-pill" x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stopColor="#f3f4f6" />
            <stop offset="50%" stopColor="#ffffff" />
            <stop offset="100%" stopColor="#f3f4f6" />
          </linearGradient>
          <linearGradient id="nx-link-line" x1="240" y1="0" x2="240" y2="1">
            <stop offset="0%" stopColor="#5a9e2e" stopOpacity="0.5" />
            <stop offset="100%" stopColor="#9ca3af" stopOpacity="0.35" />
          </linearGradient>
          <filter id="nx-soft-shadow" x="-8%" y="-8%" width="116%" height="116%">
            <feDropShadow dx="0" dy="10" stdDeviation="14" floodColor="#0f172a" floodOpacity="0.07" />
          </filter>
          <filter id="nx-card-shadow" x="-12%" y="-12%" width="124%" height="124%">
            <feDropShadow dx="0" dy="3" stdDeviation="5" floodColor="#0f172a" floodOpacity="0.06" />
          </filter>
          <marker id="nx-arrow" markerWidth="7" markerHeight="7" refX="5.5" refY="3.5" orient="auto">
            <path d="M0 0 L7 3.5 L0 7 Z" fill="#9ca3af" />
          </marker>
        </defs>

        {/* Canvas */}
        <rect x="0" y="0" width="480" height="548" rx="24" fill="url(#nx-diagram-bg)" />

        {/* Local core panel */}
        <g filter="url(#nx-soft-shadow)">
          <rect x="24" y="24" width="432" height="268" rx="20" fill="url(#nx-local-panel)" stroke="#e2e5ea" />
        </g>
        <circle cx="44" cy="52" r="4" fill="#5a9e2e" opacity="0.9" />
        <text x="56" y="56" className="nx-product-diagram__label">
          {copy.localCore}
        </text>
        <text x="440" y="56" textAnchor="end" className="nx-product-diagram__mono">
          DEVICE / 127.0.0.1
        </text>

        {nodes.map((node, index) => {
          const row = index < 2 ? 0 : 1;
          const col = node.col;
          const x = 40 + col * 208;
          const y = 76 + row * 88;
          return (
            <g key={node.id} filter="url(#nx-card-shadow)">
              <rect x={x} y={y} width="192" height="72" rx="14" fill="#ffffff" stroke="#e8eaee" />
              <rect x={x} y={y} width="192" height="3" rx="14" fill="#5a9e2e" opacity="0.35" />
              <text x={x + 16} y={y + 28} className="nx-product-diagram__idx">
                {node.id}
              </text>
              <text x={x + 16} y={y + 50} className="nx-product-diagram__node">
                {node.label}
              </text>
            </g>
          );
        })}

        <text x="40" y="258" className="nx-product-diagram__caption">
          {copy.localBoundary}
        </text>

        {/* Bridge */}
        <line
          x1="240"
          y1="300"
          x2="240"
          y2="332"
          stroke="url(#nx-link-line)"
          strokeWidth="2"
          strokeDasharray="4 5"
          markerEnd="url(#nx-arrow)"
        />
        <g filter="url(#nx-card-shadow)">
          <rect x="108" y="308" width="264" height="36" rx="18" fill="url(#nx-bridge-pill)" stroke="#e5e7eb" />
        </g>
        <text x="240" y="330" textAnchor="middle" className="nx-product-diagram__bridge">
          {copy.modelRouter}
        </text>

        {/* Optional cloud */}
        <g filter="url(#nx-soft-shadow)">
          <rect
            x="24"
            y="360"
            width="432"
            height="164"
            rx="20"
            fill="url(#nx-cloud-panel)"
            stroke="#d8dce2"
            strokeDasharray="7 5"
          />
        </g>
        <circle cx="44" cy="388" r="4" fill="#6b63d9" opacity="0.75" />
        <text x="56" y="392" className="nx-product-diagram__label nx-product-diagram__label--cloud">
          {copy.optionalCloud}
        </text>
        <text x="440" y="392" textAnchor="end" className="nx-product-diagram__mono">
          ANYCODE.WORK
        </text>

        {cloudServices.map((label, i) => {
          const widths = [104, 108, 64];
          const labels = cloudServices;
          const gap = 12;
          const total =
            widths.reduce((a, b) => a + b, 0) + gap * (labels.length - 1);
          let x = 240 - total / 2;
          for (let j = 0; j < i; j++) x += widths[j]! + gap;
          const w = widths[i]!;
          return (
            <g key={label} filter="url(#nx-card-shadow)">
              <rect
                x={x}
                y={412}
                width={w}
                height="36"
                rx="18"
                fill="#ffffff"
                stroke="#e2e5ea"
              />
              <text
                x={x + w / 2}
                y={434}
                textAnchor="middle"
                className={label === "API" ? "nx-product-diagram__cloud-api" : "nx-product-diagram__cloud-node"}
              >
                {label}
              </text>
            </g>
          );
        })}

        <text x="40" y="502" className="nx-product-diagram__caption">
          {copy.cloudBoundary}
        </text>
      </svg>
    </figure>
  );
}
