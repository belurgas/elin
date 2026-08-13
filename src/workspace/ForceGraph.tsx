import { useEffect, useRef } from "react";
import type { GraphNode, ModuleGraph } from "../types";

const ROLE_FILL: Record<string, string> = {
  module: "#a78bfa",
  genserver: "#34d399",
  supervisor: "#fbbf24",
  liveview: "#fb7185",
  controller: "#c4b5fd",
  schema: "#67e8f9",
  router: "#f9a8d4",
  agent: "#86efac",
  test: "#c9bfd9",
};

const GIT_RING: Record<string, string> = {
  added: "#34d399",
  modified: "#fbbf24",
  deleted: "#fb7185",
  untracked: "#c9bfd9",
  renamed: "#a78bfa",
};

const EDGE_COLOR: Record<string, string> = {
  alias: "#a78bfa",
  import: "#fb7185",
  use: "#34d399",
  call: "#6d5a8a",
  delegate: "#fbbf24",
  require: "#67e8f9",
};

type Sim = {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  fx: number | null;
  fy: number | null;
  r: number;
  node: GraphNode;
};

export function ForceGraph({
  graph,
  selectedId,
  active = true,
  onSelect,
  onMenu,
}: {
  graph: ModuleGraph | null;
  selectedId?: string | null;
  active?: boolean;
  onSelect?: (node: GraphNode | null) => void;
  onMenu?: (node: GraphNode, x: number, y: number) => void;
}) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const simRef = useRef<Sim[]>([]);
  const edgeRef = useRef<Array<{ from: string; to: string; kind: string; isNew?: boolean }>>([]);
  const viewRef = useRef({ x: 0, y: 0, k: 1, w: 800, h: 600 });
  const dragRef = useRef<{ id: string | null; pan: boolean; lx: number; ly: number }>({
    id: null,
    pan: false,
    lx: 0,
    ly: 0,
  });
  const selectedRef = useRef(selectedId);
  selectedRef.current = selectedId;
  const onSelectRef = useRef(onSelect);
  onSelectRef.current = onSelect;
  const onMenuRef = useRef(onMenu);
  onMenuRef.current = onMenu;
  const activeRef = useRef(active);
  activeRef.current = active;
  const fittedRef = useRef(false);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap || !graph) return;
    const prev = new Map(simRef.current.map((n) => [n.id, n]));
    const w = Math.max(wrap.clientWidth, 200);
    const h = Math.max(wrap.clientHeight, 200);
    viewRef.current.w = w;
    viewRef.current.h = h;
    const cx = w / 2;
    const cy = h / 2;
    const n = graph.nodes.length || 1;
    simRef.current = graph.nodes.map((node, i) => {
      const old = prev.get(node.id);
      if (old) {
        return { ...old, node, r: node.kind === "test" ? 10 : 14 };
      }
      const angle = (i / n) * Math.PI * 2;
      const radius = Math.min(w, h) * 0.28;
      return {
        id: node.id,
        x: cx + Math.cos(angle) * radius,
        y: cy + Math.sin(angle) * radius,
        vx: 0,
        vy: 0,
        fx: null,
        fy: null,
        r: node.kind === "test" ? 10 : 14,
        node,
      };
    });
    edgeRef.current = graph.edges.map((e) => ({
      from: e.from,
      to: e.to,
      kind: e.kind,
      isNew: e.isNew,
    }));
    if (prev.size === 0) {
      viewRef.current.x = 0;
      viewRef.current.y = 0;
      viewRef.current.k = 1;
      fittedRef.current = false;
    }
  }, [graph]);

  useEffect(() => {
    const wrap = wrapRef.current;
    const svg = svgRef.current;
    if (!wrap || !svg) return;
    let frame = 0;
    let running = true;

    const resize = () => {
      const nw = Math.max(wrap.clientWidth, 200);
      const nh = Math.max(wrap.clientHeight, 200);
      if (Math.abs(nw - viewRef.current.w) > 80 || Math.abs(nh - viewRef.current.h) > 80) {
        fittedRef.current = false;
      }
      viewRef.current.w = nw;
      viewRef.current.h = nh;
      svg.setAttribute("viewBox", `0 0 ${nw} ${nh}`);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);

    const tick = () => {
      if (!running) return;
      const { w, h } = viewRef.current;
      const nodes = simRef.current;
      const edges = edgeRef.current;
      if (activeRef.current && w > 80 && h > 80 && nodes.length) {
        const alpha = 0.12;
        for (let i = 0; i < nodes.length; i++) {
          for (let j = i + 1; j < nodes.length; j++) {
            const a = nodes[i];
            const b = nodes[j];
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            const dist = Math.hypot(dx, dy) || 0.01;
            const min = a.r + b.r + 36;
            const force = ((min * min) / (dist * dist)) * 0.55;
            dx = (dx / dist) * force;
            dy = (dy / dist) * force;
            a.vx += dx;
            a.vy += dy;
            b.vx -= dx;
            b.vy -= dy;
          }
        }
        const byId = new Map(nodes.map((n) => [n.id, n]));
        for (const edge of edges) {
          const a = byId.get(edge.from);
          const b = byId.get(edge.to);
          if (!a || !b) continue;
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const dist = Math.hypot(dx, dy) || 0.01;
          const k = 0.02 * (dist - 90);
          a.vx += (dx / dist) * k;
          a.vy += (dy / dist) * k;
          b.vx -= (dx / dist) * k;
          b.vy -= (dy / dist) * k;
        }
        const pad = 48;
        for (const node of nodes) {
          node.vx += (w / 2 - node.x) * 0.02;
          node.vy += (h / 2 - node.y) * 0.02;
          node.vx *= 0.78;
          node.vy *= 0.78;
          if (node.fx != null) {
            node.x = node.fx;
            node.vx = 0;
          } else {
            node.x += node.vx * alpha * 6;
          }
          if (node.fy != null) {
            node.y = node.fy;
            node.vy = 0;
          } else {
            node.y += node.vy * alpha * 6;
          }
          node.x = Math.min(w - pad, Math.max(pad, node.x));
          node.y = Math.min(h - pad, Math.max(pad, node.y));
        }
        if (!fittedRef.current) {
          fitView(nodes, viewRef.current);
          fittedRef.current = true;
        }
        paint(svg, nodes, edges, viewRef.current, selectedRef.current);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => {
      running = false;
      cancelAnimationFrame(frame);
      ro.disconnect();
    };
  }, [graph]);

  function clientToWorld(clientX: number, clientY: number) {
    const svg = svgRef.current;
    if (!svg) return { x: 0, y: 0 };
    const rect = svg.getBoundingClientRect();
    const v = viewRef.current;
    return {
      x: (clientX - rect.left - v.x) / v.k,
      y: (clientY - rect.top - v.y) / v.k,
    };
  }

  function hitNode(x: number, y: number) {
    let best: Sim | null = null;
    let bestD = 22;
    for (const node of simRef.current) {
      const d = Math.hypot(node.x - x, node.y - y);
      if (d < bestD) {
        bestD = d;
        best = node;
      }
    }
    return best;
  }

  return (
    <div ref={wrapRef} className="graph-stage relative h-full min-h-0 w-full overflow-hidden">
      <svg
        ref={svgRef}
        className="h-full w-full cursor-grab active:cursor-grabbing"
        onPointerDown={(e) => {
          const world = clientToWorld(e.clientX, e.clientY);
          const hit = hitNode(world.x, world.y);
          if (hit) {
            dragRef.current = { id: hit.id, pan: false, lx: e.clientX, ly: e.clientY };
            hit.fx = hit.x;
            hit.fy = hit.y;
            onSelectRef.current?.(hit.node);
            (e.target as Element).setPointerCapture?.(e.pointerId);
          } else {
            dragRef.current = { id: null, pan: true, lx: e.clientX, ly: e.clientY };
            onSelectRef.current?.(null);
          }
        }}
        onPointerMove={(e) => {
          const drag = dragRef.current;
          if (drag.id) {
            const world = clientToWorld(e.clientX, e.clientY);
            const node = simRef.current.find((n) => n.id === drag.id);
            if (node) {
              node.fx = world.x;
              node.fy = world.y;
            }
          } else if (drag.pan && (e.buttons & 1)) {
            viewRef.current.x += e.clientX - drag.lx;
            viewRef.current.y += e.clientY - drag.ly;
            drag.lx = e.clientX;
            drag.ly = e.clientY;
          }
        }}
        onPointerUp={() => {
          dragRef.current = { id: null, pan: false, lx: 0, ly: 0 };
        }}
        onDoubleClick={(e) => {
          const world = clientToWorld(e.clientX, e.clientY);
          const hit = hitNode(world.x, world.y);
          if (hit) {
            hit.fx = null;
            hit.fy = null;
          } else {
            viewRef.current.x = 0;
            viewRef.current.y = 0;
            viewRef.current.k = 1;
          }
        }}
        onWheel={(e) => {
          e.preventDefault();
          const v = viewRef.current;
          const factor = e.deltaY < 0 ? 1.08 : 0.92;
          const next = Math.min(2.4, Math.max(0.35, v.k * factor));
          const rect = svgRef.current?.getBoundingClientRect();
          if (!rect) return;
          const px = e.clientX - rect.left;
          const py = e.clientY - rect.top;
          const wx = (px - v.x) / v.k;
          const wy = (py - v.y) / v.k;
          v.k = next;
          v.x = px - wx * v.k;
          v.y = py - wy * v.k;
        }}
        onContextMenu={(e) => {
          e.preventDefault();
          const world = clientToWorld(e.clientX, e.clientY);
          const hit = hitNode(world.x, world.y);
          if (hit) {
            onSelectRef.current?.(hit.node);
            onMenuRef.current?.(hit.node, e.clientX, e.clientY);
          }
        }}
      />
    </div>
  );
}

function fitView(
  nodes: Sim[],
  view: { x: number; y: number; k: number; w: number; h: number },
) {
  if (!nodes.length) return;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const n of nodes) {
    minX = Math.min(minX, n.x - 40);
    minY = Math.min(minY, n.y - 40);
    maxX = Math.max(maxX, n.x + 40);
    maxY = Math.max(maxY, n.y + 40);
  }
  const bw = Math.max(maxX - minX, 80);
  const bh = Math.max(maxY - minY, 80);
  const k = Math.min(view.w / bw, view.h / bh, 1.35);
  view.k = k;
  view.x = view.w / 2 - k * ((minX + maxX) / 2);
  view.y = view.h / 2 - k * ((minY + maxY) / 2);
}

function paint(
  svg: SVGSVGElement,
  nodes: Sim[],
  edges: Array<{ from: string; to: string; kind: string; isNew?: boolean }>,
  view: { x: number; y: number; k: number; w: number; h: number },
  selectedId?: string | null,
) {
  let root = svg.querySelector("g.graph-root") as SVGGElement | null;
  if (!root) {
    svg.replaceChildren();
    root = document.createElementNS("http://www.w3.org/2000/svg", "g");
    root.setAttribute("class", "graph-root");
    const edgeLayer = document.createElementNS("http://www.w3.org/2000/svg", "g");
    edgeLayer.setAttribute("class", "graph-edges");
    const nodeLayer = document.createElementNS("http://www.w3.org/2000/svg", "g");
    nodeLayer.setAttribute("class", "graph-nodes");
    root.append(edgeLayer, nodeLayer);
    svg.append(root);
  }
  root.setAttribute("transform", `translate(${view.x} ${view.y}) scale(${view.k})`);
  const edgeLayer = root.querySelector(".graph-edges") as SVGGElement;
  const nodeLayer = root.querySelector(".graph-nodes") as SVGGElement;
  const byId = new Map(nodes.map((n) => [n.id, n]));

  const edgeKey = (e: (typeof edges)[number], i: number) => `${e.from}|${e.to}|${e.kind}|${i}`;
  const existingEdges = new Map(
    [...edgeLayer.children].map((el) => [el.getAttribute("data-key") ?? "", el as SVGLineElement]),
  );
  const keepEdges = new Set<string>();
  edges.forEach((edge, i) => {
    const a = byId.get(edge.from);
    const b = byId.get(edge.to);
    if (!a || !b) return;
    const key = edgeKey(edge, i);
    keepEdges.add(key);
    let line = existingEdges.get(key);
    if (!line) {
      line = document.createElementNS("http://www.w3.org/2000/svg", "line");
      line.setAttribute("data-key", key);
      line.setAttribute("stroke-linecap", "round");
      edgeLayer.append(line);
    }
    line.setAttribute("x1", String(a.x));
    line.setAttribute("y1", String(a.y));
    line.setAttribute("x2", String(b.x));
    line.setAttribute("y2", String(b.y));
    line.setAttribute("stroke", EDGE_COLOR[edge.kind] ?? "#6d5a8a");
    line.setAttribute("stroke-opacity", edge.isNew ? "0.95" : "0.42");
    line.setAttribute("stroke-width", edge.isNew ? "2.2" : "1.35");
    line.setAttribute("class", edge.isNew ? "graph-edge-flow" : "");
  });
  for (const [key, el] of existingEdges) {
    if (!keepEdges.has(key)) el.remove();
  }

  const existingNodes = new Map(
    [...nodeLayer.children].map((el) => [el.getAttribute("data-id") ?? "", el as SVGGElement]),
  );
  const keepNodes = new Set<string>();
  for (const sim of nodes) {
    keepNodes.add(sim.id);
    let g = existingNodes.get(sim.id);
    if (!g) {
      g = document.createElementNS("http://www.w3.org/2000/svg", "g");
      g.setAttribute("data-id", sim.id);
      const glow = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      glow.setAttribute("class", "graph-glow");
      const ring = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      ring.setAttribute("class", "graph-ring");
      const core = document.createElementNS("http://www.w3.org/2000/svg", "circle");
      core.setAttribute("class", "graph-core");
      const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
      label.setAttribute("class", "graph-label");
      label.setAttribute("text-anchor", "middle");
      label.setAttribute("fill", "#e8e2f7");
      label.setAttribute("font-size", "11");
      label.setAttribute("font-family", "Outfit, sans-serif");
      g.append(glow, ring, core, label);
      nodeLayer.append(g);
    }
    const selected = selectedId === sim.id;
    const role = sim.node.role || (sim.node.kind === "test" ? "test" : "module");
    const fill = ROLE_FILL[role] ?? ROLE_FILL.module;
    const git = sim.node.git ?? "";
    const glow = g.querySelector(".graph-glow") as SVGCircleElement;
    const ring = g.querySelector(".graph-ring") as SVGCircleElement;
    const core = g.querySelector(".graph-core") as SVGCircleElement;
    const label = g.querySelector(".graph-label") as SVGTextElement;
    const r = selected ? sim.r + 4 : sim.r;
    glow.setAttribute("cx", String(sim.x));
    glow.setAttribute("cy", String(sim.y));
    glow.setAttribute("r", String(r + 10));
    glow.setAttribute("fill", fill);
    glow.setAttribute("fill-opacity", selected ? "0.28" : git ? "0.14" : "0.06");
    ring.setAttribute("cx", String(sim.x));
    ring.setAttribute("cy", String(sim.y));
    ring.setAttribute("r", String(r + 3));
    ring.setAttribute("fill", "none");
    ring.setAttribute("stroke", GIT_RING[git] ?? (selected ? fill : "transparent"));
    ring.setAttribute("stroke-width", git || selected ? "2" : "0");
    ring.setAttribute("stroke-dasharray", git === "untracked" || git === "deleted" ? "4 3" : "");
    core.setAttribute("cx", String(sim.x));
    core.setAttribute("cy", String(sim.y));
    core.setAttribute("r", String(r));
    core.setAttribute("fill", "#1b1828");
    core.setAttribute("stroke", fill);
    core.setAttribute("stroke-width", selected ? "2.4" : "1.6");
    label.setAttribute("x", String(sim.x));
    label.setAttribute("y", String(sim.y + r + 16));
    label.textContent = sim.node.label;
    g.setAttribute("class", sim.fx != null ? "graph-node is-pinned" : "graph-node");
  }
  for (const [id, el] of existingNodes) {
    if (!keepNodes.has(id)) el.remove();
  }
}
