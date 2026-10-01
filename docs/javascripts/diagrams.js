/* Wide mermaid diagrams.
 *
 * Material renders each diagram into a closed shadow root, and mermaid scales the SVG
 * down to the column width, so a long left-to-right flowchart ends up with unreadable
 * text. This script wraps every rendered diagram in a .huv-diagram container (see
 * extra.css) and, when the fitted diagram would be drawn at less than MIN_SCALE of its
 * natural size, gives it a minimum width (at most MAX_STRETCH column widths) so it
 * scrolls sideways instead of shrinking.
 * The natural size is measured from outside the shadow root by briefly letting the
 * diagram grow to its full width. The container is keyboard-focusable while it
 * overflows, so it can be scrolled without a mouse.
 */
(function () {
  "use strict";

  var MIN_SCALE = 0.7; // smallest drawing scale before the diagram scrolls instead
  var MAX_STRETCH = 4; // never make a diagram wider than this many column widths
  var observer = null;
  var resizer = typeof ResizeObserver === "function"
    ? new ResizeObserver(function (entries) {
        entries.forEach(function (entry) { fit(entry.target); });
      })
    : null;

  function fit(box) {
    var host = box.firstElementChild;
    if (!host) return;
    host.style.minWidth = "";
    var fitted = host.getBoundingClientRect();
    if (fitted.height > 0 && fitted.width > 0) {
      host.style.width = "10000px"; // wider than any diagram: the SVG stops at its natural size
      var natural = host.getBoundingClientRect().height;
      host.style.width = "";
      var scale = fitted.height / natural;
      if (natural > 0 && scale < MIN_SCALE - 0.01) {
        var width = Math.min((fitted.width * MIN_SCALE) / scale, box.clientWidth * MAX_STRETCH);
        if (width > box.clientWidth + 1) host.style.minWidth = Math.ceil(width) + "px";
      }
    }
    if (box.scrollWidth > box.clientWidth + 1) {
      box.setAttribute("tabindex", "0");
      box.setAttribute("aria-label", "Diagram (scrolls sideways)");
    } else {
      box.removeAttribute("tabindex");
      box.setAttribute("aria-label", "Diagram");
    }
  }

  function wrap(host) {
    var parent = host.parentElement;
    if (!parent || parent.classList.contains("huv-diagram")) return;
    var box = document.createElement("div");
    box.className = "huv-diagram";
    box.setAttribute("role", "group");
    parent.insertBefore(box, host);
    box.appendChild(host);
    if (resizer) resizer.observe(box);
    fit(box);
  }

  function scan(root) {
    if (root.matches && root.matches("div.mermaid")) {
      wrap(root);
      return;
    }
    if (root.querySelectorAll) root.querySelectorAll("div.mermaid").forEach(wrap);
  }

  function init() {
    var content = document.querySelector(".md-content") || document.body;
    if (observer) observer.disconnect();
    scan(content);
    observer = new MutationObserver(function (records) {
      records.forEach(function (record) {
        record.addedNodes.forEach(function (node) {
          if (node.nodeType === 1) scan(node);
        });
      });
    });
    observer.observe(content, { childList: true, subtree: true });
  }

  if (typeof document$ !== "undefined") {
    document$.subscribe(init);
  } else if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
