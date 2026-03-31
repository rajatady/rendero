import { h, useState, mount } from "./renderer.js";
function Button({ title, color = "#007AFF", onPress }) {
  return /* @__PURE__ */ h(
    "frame",
    {
      width: 200,
      height: 38,
      backgroundColor: color,
      borderRadius: 8,
      flexDirection: "row",
      padding: 8,
      onClick: onPress
    },
    /* @__PURE__ */ h("text", { text: title, fontSize: 13, color: "#ffffff", fontWeight: 700 })
  );
}
function computeLayout(container, children) {
  const { width, height, padding = 0, gap = 0, flexDirection = "column" } = container;
  const isRow = flexDirection === "row";
  const innerW = width - padding * 2;
  const innerH = height - padding * 2;
  const mainSize = isRow ? innerW : innerH;
  const crossSize = isRow ? innerH : innerW;
  let fixedTotal = 0;
  let flexTotal = 0;
  const gapTotal = Math.max(0, children.length - 1) * gap;
  for (const child of children) {
    if (child.flex) {
      flexTotal += child.flex;
    } else {
      fixedTotal += isRow ? child.width || 0 : child.height || 0;
    }
  }
  const remaining = mainSize - fixedTotal - gapTotal;
  const perFlex = flexTotal > 0 ? remaining / flexTotal : 0;
  let offset = padding;
  const results = [];
  for (const child of children) {
    const childMain = child.flex ? perFlex * child.flex : isRow ? child.width : child.height;
    const childCross = isRow ? child.height || crossSize : child.width || crossSize;
    results.push({
      ...child,
      computedX: isRow ? offset : padding,
      computedY: isRow ? padding : offset,
      computedW: isRow ? childMain : childCross,
      computedH: isRow ? childCross : childMain
    });
    offset += childMain + gap;
  }
  return results;
}
const LAYOUTS = [
  {
    name: "Column + Fixed Heights",
    desc: "Three children with fixed heights, stacked vertically.",
    container: { width: 300, height: 300, padding: 16, gap: 10, flexDirection: "column" },
    children: [
      { label: "Header", height: 50, color: "#e74c3c" },
      { label: "Content", height: 100, color: "#3498db" },
      { label: "Footer", height: 50, color: "#2ecc71" }
    ]
  },
  {
    name: "Column + Flex",
    desc: "Header is fixed, Content fills remaining space.",
    container: { width: 300, height: 300, padding: 16, gap: 10, flexDirection: "column" },
    children: [
      { label: "Header", height: 50, color: "#e74c3c" },
      { label: "Content (flex:1)", flex: 1, color: "#3498db" },
      { label: "Footer", height: 50, color: "#2ecc71" }
    ]
  },
  {
    name: "Row Layout",
    desc: "Three items side by side, middle one flexes.",
    container: { width: 300, height: 200, padding: 12, gap: 8, flexDirection: "row" },
    children: [
      { label: "L", width: 60, color: "#9b59b6" },
      { label: "Center (flex:1)", flex: 1, color: "#3498db" },
      { label: "R", width: 60, color: "#e67e22" }
    ]
  },
  {
    name: "Flex Ratios",
    desc: "flex:1 vs flex:2 \u2014 second child gets 2x the space.",
    container: { width: 300, height: 200, padding: 12, gap: 8, flexDirection: "row" },
    children: [
      { label: "flex:1", flex: 1, color: "#e74c3c" },
      { label: "flex:2", flex: 2, color: "#3498db" },
      { label: "flex:1", flex: 1, color: "#2ecc71" }
    ]
  },
  {
    name: "Nested Layout",
    desc: "Column with a row inside \u2014 like a real app.",
    container: { width: 300, height: 300, padding: 16, gap: 10, flexDirection: "column" },
    children: [
      { label: "Title Bar", height: 44, color: "#34495e" },
      {
        label: "Button Row (flex:1)",
        flex: 1,
        color: "#ecf0f1",
        isRow: true,
        subChildren: [
          { label: "A", flex: 1, color: "#e74c3c" },
          { label: "B", flex: 1, color: "#3498db" },
          { label: "C", flex: 1, color: "#2ecc71" }
        ]
      },
      { label: "Status Bar", height: 30, color: "#7f8c8d" }
    ]
  }
];
function App() {
  const [layoutIdx, setLayoutIdx] = useState(0);
  const layout = LAYOUTS[layoutIdx];
  const computed = computeLayout(layout.container, layout.children);
  const el = document.getElementById("step-label");
  if (el) el.textContent = `Yoga: ${layout.name}`;
  const ctr = layout.container;
  return /* @__PURE__ */ h("frame", { x: 20, y: 40, width: 330, height: 800, flexDirection: "column", gap: 12 }, /* @__PURE__ */ h(
    "frame",
    {
      width: 310,
      height: 70,
      backgroundColor: "#ffffff",
      borderRadius: 12,
      flexDirection: "column",
      padding: 14,
      gap: 4
    },
    /* @__PURE__ */ h("text", { text: "Yoga Layout Algorithm", fontSize: 18, color: "#1a1a2e", fontWeight: 700 }),
    /* @__PURE__ */ h("text", { text: layout.desc, fontSize: 11, color: "#666" })
  ), /* @__PURE__ */ h("frame", { width: ctr.width, height: ctr.height, backgroundColor: "#dfe6e9", borderRadius: 8 }, computed.map((child) => {
    if (child.subChildren) {
      const subComputed = computeLayout(
        { width: child.computedW, height: child.computedH, padding: 4, gap: 4, flexDirection: "row" },
        child.subChildren
      );
      return /* @__PURE__ */ h(
        "frame",
        {
          x: child.computedX,
          y: child.computedY,
          width: child.computedW,
          height: child.computedH,
          backgroundColor: child.color,
          borderRadius: 4
        },
        subComputed.map(
          (sub) => /* @__PURE__ */ h(
            "frame",
            {
              x: sub.computedX,
              y: sub.computedY,
              width: sub.computedW,
              height: sub.computedH,
              backgroundColor: sub.color,
              borderRadius: 4
            },
            /* @__PURE__ */ h("text", { text: sub.label, fontSize: 11, color: "#fff", fontWeight: 600 })
          )
        )
      );
    }
    return /* @__PURE__ */ h(
      "frame",
      {
        x: child.computedX,
        y: child.computedY,
        width: child.computedW,
        height: child.computedH,
        backgroundColor: child.color,
        borderRadius: 4
      },
      /* @__PURE__ */ h("text", { text: child.label, fontSize: 12, color: "#ffffff", fontWeight: 600 })
    );
  })), /* @__PURE__ */ h(
    "frame",
    {
      width: 310,
      height: 200,
      backgroundColor: "#ffffff",
      borderRadius: 12,
      flexDirection: "column",
      padding: 14,
      gap: 6
    },
    /* @__PURE__ */ h("text", { text: "Computed Positions", fontSize: 14, color: "#1a1a2e", fontWeight: 700 }),
    computed.map(
      (child) => /* @__PURE__ */ h(
        "text",
        {
          text: `${child.label}: x=${Math.round(child.computedX)} y=${Math.round(child.computedY)} w=${Math.round(child.computedW)} h=${Math.round(child.computedH)}`,
          fontSize: 11,
          color: "#333"
        }
      )
    ),
    /* @__PURE__ */ h(
      "text",
      {
        text: `Container: ${ctr.width}x${ctr.height}, pad=${ctr.padding}, gap=${ctr.gap}, dir=${ctr.flexDirection}`,
        fontSize: 10,
        color: "#999"
      }
    )
  ), /* @__PURE__ */ h("frame", { width: 310, height: 44, flexDirection: "row", gap: 8 }, /* @__PURE__ */ h(
    Button,
    {
      title: "< Prev",
      color: "#666",
      onPress: () => setLayoutIdx((layoutIdx - 1 + LAYOUTS.length) % LAYOUTS.length)
    }
  ), /* @__PURE__ */ h(
    Button,
    {
      title: "Next >",
      color: "#007AFF",
      onPress: () => setLayoutIdx((layoutIdx + 1) % LAYOUTS.length)
    }
  )));
}
mount(App, document.getElementById("canvas"));
