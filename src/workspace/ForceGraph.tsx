import { useEffect, useRef, useState } from "react";
import type { GraphEdge, GraphNode, ModuleGraph } from "../types";

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
  delegate: "#fbbf24",
  require: "#67e8f9",
  call: "#4a3d63",
};

const STRUCTURAL = new Set(["alias", "import", "use", "delegate", "require"]);

type Sim = {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  ax: number;
  ay: number;
  degree: number;
  label: string;
  important: boolean;
  node: GraphNode;
};

type View = { x: number; y: number; k: number; w: number; h: number };

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
  const edgeRef = useRef<GraphEdge[]>([]);
  const layoutEdgeRef = useRef<GraphEdge[]>([]);
  const viewRef = useRef<View>({ x: 0, y: 0, k: 1, w: 800, h: 600 });
  const alphaRef = useRef(1);
  const alphaTargetRef = useRef(0);
  const settledRef = useRef(false);
  const fitUntilRef = useRef(0);
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
  const hoverRef = useRef<string | null>(null);
  const [tip, setTip] = useState<{ id: string; x: number; y: number } | null>(null);
  const kickRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (!graph) {
      simRef.current = [];
      edgeRef.current = [];
      layoutEdgeRef.current = [];
      return;
    }
    const visible = visibleNodes(graph);
    const keep = new Set(visible.map((n) => n.id));
    const labels = disambiguate(visible);
    const prev = new Map(simRef.current.map((n) => [n.id, n]));
    const clusters = clusterLayout(visible);
    const next: Sim[] = visible.map((node) => {
      const old = prev.get(node.id);
      const r = radiusFor(node);
      const important = isImportant(node, visible.length);
      const key = clusterKey(node);
      const seed = clusters.anchors.get(key) ?? { x: 0, y: 0 };
      if (old) {
        return {
          ...old,
          node,
          r,
          label: labels.get(node.id) ?? node.label,
          important,
          ax: seed.x,
          ay: seed.y,
        };
      }
      const a = hash(node.id) * Math.PI * 2;
      const rad = (clusters.spread.get(key) ?? 40) * (0.15 + hash(`${node.id}:r`) * 0.85);
      return {
        id: node.id,
        x: seed.x + Math.cos(a) * rad,
        y: seed.y + Math.sin(a) * rad,
        vx: 0,
        vy: 0,
        r,
        ax: seed.x,
        ay: seed.y,
        degree: 0,
        label: labels.get(node.id) ?? node.label,
        important,
        node,
      };
    });
    const edges = graph.edges.filter((e) => keep.has(e.from) && keep.has(e.to));
    const layout = edges.filter((e) => STRUCTURAL.has(e.kind));
    const degree = new Map<string, number>();
    for (const e of layout) {
      degree.set(e.from, (degree.get(e.from) ?? 0) + 1);
      degree.set(e.to, (degree.get(e.to) ?? 0) + 1);
    }
    for (const node of next) node.degree = degree.get(node.id) ?? 0;

    const membershipChanged = prev.size !== next.length || next.some((n) => !prev.has(n.id));
    simRef.current = next;
    edgeRef.current = edges;
    layoutEdgeRef.current = layout;
    if (membershipChanged) {
      alphaRef.current = 1;
      alphaTargetRef.current = 0;
      settledRef.current = false;
      fitUntilRef.current = 48;
      fittedNow(next, viewRef.current);
    }
    kickRef.current?.();
  }, [graph]);

  useEffect(() => {
    const wrapEl = wrapRef.current;
    const svgEl = svgRef.current;
    if (!wrapEl || !svgEl) return;
    const wrap: HTMLDivElement = wrapEl;
    const svg: SVGSVGElement = svgEl;
    let frame = 0;
    let running = true;
    let dirty = true;

    function tick() {
      frame = 0;
      if (!running) return;
      const nodes = simRef.current;
      if (!activeRef.current || !nodes.length) return;
      const dragging = Boolean(dragRef.current.id);
      const panning = dragRef.current.pan;
      if (!settledRef.current || dragging || dirty) {
        if (!settledRef.current || dragging) {
          const next = step(nodes, layoutEdgeRef.current, {
            alpha: alphaRef.current,
            alphaTarget: dragging ? 0.22 : alphaTargetRef.current,
            dragging,
            dragId: dragRef.current.id,
            view: viewRef.current,
            fitLeft: fitUntilRef.current,
          });
          alphaRef.current = next.alpha;
          settledRef.current = next.settled;
          fitUntilRef.current = next.fitLeft;
        }
        paint(svg, nodes, edgeRef.current, viewRef.current, selectedRef.current, hoverRef.current);
        dirty = false;
      }
      if (!settledRef.current || dragging || panning) kick();
    }

    function kick() {
      if (frame) return;
      frame = requestAnimationFrame(tick);
    }
    kickRef.current = kick;

    const resize = () => {
      viewRef.current.w = Math.max(wrap.clientWidth, 200);
      viewRef.current.h = Math.max(wrap.clientHeight, 200);
      svg.setAttribute("viewBox", `0 0 ${viewRef.current.w} ${viewRef.current.h}`);
      if (simRef.current.length) fittedNow(simRef.current, viewRef.current);
      dirty = true;
      kick();
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(wrap);
    return () => {
      running = false;
      kickRef.current = null;
      cancelAnimationFrame(frame);
      ro.disconnect();
    };
  }, []);

  useEffect(() => {
    if (active) kickRef.current?.();
  }, [active]);

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg || !simRef.current.length) return;
    paint(svg, simRef.current, edgeRef.current, viewRef.current, selectedId, hoverRef.current);
  }, [selectedId]);

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
    let bestD = 26;
    for (const node of simRef.current) {
      const d = Math.hypot(node.x - x, node.y - y);
      if (d < Math.max(bestD, node.r + 10)) {
        bestD = d;
        best = node;
      }
    }
    return best;
  }

  function heat() {
    settledRef.current = false;
    alphaTargetRef.current = 0.22;
    kickRef.current?.();
  }

  function coolSoon() {
    alphaTargetRef.current = 0;
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
            onSelectRef.current?.(hit.node);
            heat();
          } else {
            dragRef.current = { id: null, pan: true, lx: e.clientX, ly: e.clientY };
            onSelectRef.current?.(null);
          }
          (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
        }}
        onPointerMove={(e) => {
          const drag = dragRef.current;
          if (drag.id) {
            const world = clientToWorld(e.clientX, e.clientY);
            const node = simRef.current.find((n) => n.id === drag.id);
            if (node) {
              node.x = world.x;
              node.y = world.y;
              node.vx = 0;
              node.vy = 0;
            }
            return;
          }
          if (drag.pan && e.buttons & 1) {
            viewRef.current.x += e.clientX - drag.lx;
            viewRef.current.y += e.clientY - drag.ly;
            drag.lx = e.clientX;
            drag.ly = e.clientY;
            kickRef.current?.();
            return;
          }
          const world = clientToWorld(e.clientX, e.clientY);
          const hit = hitNode(world.x, world.y);
          const next = hit?.id ?? null;
          if (hoverRef.current !== next) {
            hoverRef.current = next;
            const svg = svgRef.current;
            if (svg) {
              paint(svg, simRef.current, edgeRef.current, viewRef.current, selectedRef.current, next);
            }
            if (hit) {
              const v = viewRef.current;
              setTip({
                id: hit.node.id,
                x: hit.x * v.k + v.x + 16,
                y: hit.y * v.k + v.y - 8,
              });
            } else {
              setTip(null);
            }
          }
        }}
        onPointerUp={(e) => {
          dragRef.current = { id: null, pan: false, lx: 0, ly: 0 };
          coolSoon();
          (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
        }}
        onPointerCancel={(e) => {
          dragRef.current = { id: null, pan: false, lx: 0, ly: 0 };
          coolSoon();
          (e.currentTarget as Element).releasePointerCapture?.(e.pointerId);
        }}
        onPointerLeave={() => {
          hoverRef.current = null;
          setTip(null);
        }}
        onDoubleClick={(e) => {
          const world = clientToWorld(e.clientX, e.clientY);
          const hit = hitNode(world.x, world.y);
          if (!hit) fittedNow(simRef.current, viewRef.current);
          kickRef.current?.();
        }}
        onWheel={(e) => {
          e.preventDefault();
          const v = viewRef.current;
          const factor = e.deltaY < 0 ? 1.08 : 0.92;
          const next = Math.min(3.2, Math.max(0.18, v.k * factor));
          const rect = svgRef.current?.getBoundingClientRect();
          if (!rect) return;
          const px = e.clientX - rect.left;
          const py = e.clientY - rect.top;
          const wx = (px - v.x) / v.k;
          const wy = (py - v.y) / v.k;
          v.k = next;
          v.x = px - wx * v.k;
          v.y = py - wy * v.k;
          const svg = svgRef.current;
          if (svg) paint(svg, simRef.current, edgeRef.current, v, selectedRef.current, hoverRef.current);
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
      {tip ? (
        <div
          className="graph-tip pointer-events-none absolute z-10 max-w-sm truncate rounded-md px-2 py-1 font-mono text-[11px]"
          style={{ left: tip.x, top: tip.y }}
        >
          {tip.id}
        </div>
      ) : null}
    </div>
  );
}

function step(
  nodes: Sim[],
  edges: GraphEdge[],
  state: {
    alpha: number;
    alphaTarget: number;
    dragging: boolean;
    dragId: string | null;
    view: View;
    fitLeft: number;
  },
) {
  let alpha = state.alpha + (state.alphaTarget - state.alpha) * 0.08;
  if (alpha < 0.012 && state.alphaTarget === 0 && !state.dragging) {
    for (const node of nodes) {
      node.vx = 0;
      node.vy = 0;
    }
    return { alpha, settled: true, fitLeft: 0 };
  }

  const byId = new Map(nodes.map((n) => [n.id, n]));
  manyBody(nodes, alpha);
  collide(nodes, alpha);

  for (const edge of edges) {
    const p = byId.get(edge.from);
    const q = byId.get(edge.to);
    if (!p || !q) continue;
    const dx = q.x - p.x;
    const dy = q.y - p.y;
    const dist = Math.hypot(dx, dy) || 0.01;
    const rest = 72 + Math.min(p.r, q.r);
    const bias = 0.045 / Math.sqrt((p.degree + 1) * (q.degree + 1));
    const k = ((dist - rest) / dist) * bias * alpha;
    p.vx += dx * k;
    p.vy += dy * k;
    q.vx -= dx * k;
    q.vy -= dy * k;
  }

  for (const node of nodes) {
    if (state.dragId === node.id) {
      node.vx = 0;
      node.vy = 0;
      continue;
    }
    node.vx += (node.ax - node.x) * 0.055 * alpha;
    node.vy += (node.ay - node.y) * 0.055 * alpha;
    node.vx *= 0.62;
    node.vy *= 0.62;
    node.x += node.vx;
    node.y += node.vy;
  }

  let fitLeft = state.fitLeft;
  if (fitLeft > 0) {
    fitLeft -= 1;
    if (fitLeft % 6 === 0) fittedNow(nodes, state.view);
  }
  return { alpha, settled: false, fitLeft };
}

function manyBody(nodes: Sim[], alpha: number) {
  const n = nodes.length;
  const charge = -220 * alpha;
  const cutoff = n > 80 ? 280 : 420;
  if (n < 110) {
    for (let i = 0; i < n; i++) {
      const a = nodes[i];
      for (let j = i + 1; j < n; j++) {
        const b = nodes[j];
        const dx = a.x - b.x;
        const dy = a.y - b.y;
        const dist2 = dx * dx + dy * dy || 0.25;
        if (dist2 > cutoff * cutoff) continue;
        const dist = Math.sqrt(dist2);
        const force = Math.min(2.4, Math.abs(charge) / dist2);
        const ux = dx / dist;
        const uy = dy / dist;
        a.vx += ux * force;
        a.vy += uy * force;
        b.vx -= ux * force;
        b.vy -= uy * force;
      }
    }
    return;
  }
  const cell = 90;
  const bins = new Map<string, Sim[]>();
  for (const node of nodes) {
    const key = `${Math.floor(node.x / cell)}:${Math.floor(node.y / cell)}`;
    const list = bins.get(key);
    if (list) list.push(node);
    else bins.set(key, [node]);
  }
  for (const node of nodes) {
    const cx = Math.floor(node.x / cell);
    const cy = Math.floor(node.y / cell);
    for (let ox = -1; ox <= 1; ox++) {
      for (let oy = -1; oy <= 1; oy++) {
        const list = bins.get(`${cx + ox}:${cy + oy}`);
        if (!list) continue;
        for (const other of list) {
          if (other.id <= node.id) continue;
          const dx = node.x - other.x;
          const dy = node.y - other.y;
          const dist2 = dx * dx + dy * dy || 0.25;
          if (dist2 > cutoff * cutoff) continue;
          const dist = Math.sqrt(dist2);
          const force = Math.min(2.2, Math.abs(charge) / dist2);
          const ux = dx / dist;
          const uy = dy / dist;
          node.vx += ux * force;
          node.vy += uy * force;
          other.vx -= ux * force;
          other.vy -= uy * force;
        }
      }
    }
  }
}

function collide(nodes: Sim[], alpha: number) {
  const n = nodes.length;
  if (n >= 80) {
    collideBins(nodes, alpha);
    return;
  }
  for (let i = 0; i < n; i++) {
    const a = nodes[i];
    for (let j = i + 1; j < n; j++) {
      separate(a, nodes[j], alpha);
    }
  }
}

function collideBins(nodes: Sim[], alpha: number) {
  const cell = 80;
  const bins = new Map<string, Sim[]>();
  for (const node of nodes) {
    const key = `${Math.floor(node.x / cell)}:${Math.floor(node.y / cell)}`;
    const list = bins.get(key);
    if (list) list.push(node);
    else bins.set(key, [node]);
  }
  for (const node of nodes) {
    const cx = Math.floor(node.x / cell);
    const cy = Math.floor(node.y / cell);
    for (let ox = -1; ox <= 1; ox++) {
      for (let oy = -1; oy <= 1; oy++) {
        const list = bins.get(`${cx + ox}:${cy + oy}`);
        if (!list) continue;
        for (const other of list) {
          if (other.id <= node.id) continue;
          separate(node, other, alpha);
        }
      }
    }
  }
}

function separate(a: Sim, b: Sim, alpha: number) {
  const min = a.r + b.r + 14;
  const dx = a.x - b.x;
  if (Math.abs(dx) > min) return;
  const dy = a.y - b.y;
  if (Math.abs(dy) > min) return;
  const dist = Math.hypot(dx, dy) || 0.01;
  if (dist >= min) return;
  const push = ((min - dist) / dist) * 0.45 * (0.4 + alpha);
  a.x += dx * push;
  a.y += dy * push;
  b.x -= dx * push;
  b.y -= dy * push;
}

function fittedNow(nodes: Sim[], view: View) {
  if (!nodes.length || view.w < 40) return;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const n of nodes) {
    minX = Math.min(minX, n.x - 36);
    minY = Math.min(minY, n.y - 36);
    maxX = Math.max(maxX, n.x + 36);
    maxY = Math.max(maxY, n.y + 48);
  }
  const bw = Math.max(maxX - minX, 120);
  const bh = Math.max(maxY - minY, 120);
  const k = Math.min(view.w / bw, view.h / bh) * 0.88;
  view.k = Math.min(Math.max(k, 0.22), 1.6);
  view.x = view.w / 2 - view.k * ((minX + maxX) / 2);
  view.y = view.h / 2 - view.k * ((minY + maxY) / 2);
}

function paint(
  svg: SVGSVGElement,
  nodes: Sim[],
  edges: GraphEdge[],
  view: View,
  selectedId?: string | null,
  hoverId?: string | null,
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
  const showCalls = nodes.length < 36 && view.k > 1.1;
  const drawn = showCalls ? edges : edges.filter((e) => STRUCTURAL.has(e.kind) || e.isNew);

  const existingEdges = new Map(
    [...edgeLayer.children].map((el) => [el.getAttribute("data-key") ?? "", el as SVGLineElement]),
  );
  const keepEdges = new Set<string>();
  drawn.forEach((edge, i) => {
    const a = byId.get(edge.from);
    const b = byId.get(edge.to);
    if (!a || !b) return;
    const key = `${edge.from}|${edge.to}|${edge.kind}|${i}`;
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
    const structural = STRUCTURAL.has(edge.kind);
    line.setAttribute("stroke", EDGE_COLOR[edge.kind] ?? "#6d5a8a");
    line.setAttribute("stroke-opacity", edge.isNew ? "0.95" : structural ? "0.38" : "0.16");
    line.setAttribute("stroke-width", edge.isNew ? "2.2" : structural ? "1.25" : "0.9");
    line.setAttribute("class", edge.isNew ? "graph-edge-flow" : "");
  });
  for (const [key, el] of existingEdges) {
    if (!keepEdges.has(key)) el.remove();
  }

  const showAllLabels = nodes.length <= 28 || view.k >= 1.25;
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
    const hovered = hoverId === sim.id;
    const role = sim.node.role || (sim.node.kind === "test" ? "test" : "module");
    const fill = ROLE_FILL[role] ?? ROLE_FILL.module;
    const git = sim.node.git ?? "";
    const glow = g.querySelector(".graph-glow") as SVGCircleElement;
    const ring = g.querySelector(".graph-ring") as SVGCircleElement;
    const core = g.querySelector(".graph-core") as SVGCircleElement;
    const label = g.querySelector(".graph-label") as SVGTextElement;
    const r = selected || hovered ? sim.r + 3 : sim.r;
    glow.setAttribute("cx", String(sim.x));
    glow.setAttribute("cy", String(sim.y));
    glow.setAttribute("r", String(r + 10));
    glow.setAttribute("fill", fill);
    glow.setAttribute("fill-opacity", selected || hovered ? "0.28" : git ? "0.14" : "0.06");
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
    const showLabel = showAllLabels || sim.important || selected || hovered;
    label.setAttribute("x", String(sim.x));
    label.setAttribute("y", String(sim.y + r + 15));
    label.textContent = showLabel ? sim.label : "";
    label.setAttribute("fill-opacity", hovered || selected ? "1" : "0.82");
  }
  for (const [id, el] of existingNodes) {
    if (!keepNodes.has(id)) el.remove();
  }
}

function visibleNodes(graph: ModuleGraph): GraphNode[] {
  const lib = graph.nodes.filter((n) => n.kind !== "test");
  return lib.length >= 18 ? lib : graph.nodes;
}

function clusterKey(node: GraphNode) {
  return node.boundary || "_";
}

function clusterLayout(nodes: GraphNode[]): {
  anchors: Map<string, { x: number; y: number }>;
  spread: Map<string, number>;
} {
  const groups = new Map<string, GraphNode[]>();
  for (const node of nodes) {
    const key = clusterKey(node);
    const list = groups.get(key);
    if (list) list.push(node);
    else groups.set(key, [node]);
  }
  const keys = [...groups.keys()].sort();
  const anchors = new Map<string, { x: number; y: number }>();
  const spread = new Map<string, number>();
  for (const key of keys) {
    spread.set(key, 28 + Math.sqrt(groups.get(key)?.length ?? 1) * 26);
  }
  if (keys.length <= 1) {
    anchors.set(keys[0] ?? "_", { x: 0, y: 0 });
    return { anchors, spread };
  }
  const ring = 220 + Math.sqrt(nodes.length) * 24;
  keys.forEach((key, i) => {
    const angle = (i / keys.length) * Math.PI * 2 - Math.PI / 2;
    anchors.set(key, { x: Math.cos(angle) * ring, y: Math.sin(angle) * ring });
  });
  return { anchors, spread };
}

function disambiguate(nodes: GraphNode[]): Map<string, string> {
  const leafCount = new Map<string, number>();
  for (const node of nodes) {
    const leaf = node.id.split(".").pop() ?? node.id;
    leafCount.set(leaf, (leafCount.get(leaf) ?? 0) + 1);
  }
  const out = new Map<string, string>();
  for (const node of nodes) {
    const parts = node.id.split(".");
    const leaf = parts[parts.length - 1] ?? node.id;
    if ((leafCount.get(leaf) ?? 0) > 1 && parts.length > 1) {
      out.set(node.id, `${parts[parts.length - 2]}.${leaf}`);
    } else {
      out.set(node.id, leaf);
    }
  }
  return out;
}

function radiusFor(node: GraphNode) {
  const loc = node.loc ?? 20;
  let r = 7 + Math.min(11, Math.sqrt(loc) / 3.2);
  if (node.kind === "test") r *= 0.78;
  if (node.role === "supervisor" || node.role === "router" || /Application$/.test(node.id)) r += 3;
  return r;
}

function isImportant(node: GraphNode, total: number) {
  if (total < 36) return true;
  const role = node.role ?? "";
  if (role === "supervisor" || role === "router" || role === "liveview") return true;
  if (/Application$/.test(node.id)) return true;
  return (node.fanIn ?? 0) + (node.fanOut ?? 0) >= 5;
}

function hash(text: string) {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 16777619);
  return (h >>> 0) / 4294967296;
}
