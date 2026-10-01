/* MathJax configuration for pymdownx.arithmatex (generic mode), following the
   Material for MkDocs recipe. arithmatex wraps math in .arithmatex spans/divs using
   \( \) and \[ \] delimiters; only those elements are typeset. MathJax itself is
   loaded right after this file (see extra_javascript in mkdocs.yml). */

/* MathJax does not line-break inline formulas, and Material hides page-level horizontal
   overflow, so an inline formula wider than its text block (a long expression on a phone)
   would be cut off. Give only those formulas a horizontal scroller (.huv-math-scroll in
   extra.css). The natural width is read from MathJax's own container, which the scroller
   does not shrink, so re-measuring after a resize is stable. */
function huvFitInlineMath() {
  document.querySelectorAll(".md-typeset span.arithmatex").forEach(function (span) {
    var formula = span.querySelector("mjx-container");
    var block = span.closest("p, li, td, th, dd, figcaption, .admonition, details") || span.parentElement;
    if (!formula || !block) return;
    var style = window.getComputedStyle(block);
    var room = block.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    span.classList.toggle("huv-math-scroll", formula.getBoundingClientRect().width > room + 1);
  });
}

window.MathJax = {
  tex: {
    inlineMath: [["\\(", "\\)"]],
    displayMath: [["\\[", "\\]"]],
    processEscapes: true,
    processEnvironments: true
  },
  options: {
    ignoreHtmlClass: ".*|",
    processHtmlClass: "arithmatex"
  },
  startup: {
    pageReady: function () {
      return window.MathJax.startup.defaultPageReady().then(huvFitInlineMath);
    }
  }
};

/* Instant navigation swaps the page content without reloading scripts, so typeset
   again on every page change. The first emission is the initial page, which MathJax
   typesets on startup; typesetting it a second time would re-render MathJax's own
   hidden assistive MathML and make screen readers announce each equation twice. */
if (typeof document$ !== "undefined") {
  var huvFirstPage = true;
  document$.subscribe(function () {
    if (huvFirstPage) {
      huvFirstPage = false;
      return;
    }
    var mj = window.MathJax;
    if (!mj || !mj.startup || !mj.startup.output || !mj.typesetPromise) return;
    mj.startup.output.clearCache();
    mj.typesetClear();
    mj.texReset();
    mj.typesetPromise().then(huvFitInlineMath);
  });
}

(function () {
  var timer = null;
  window.addEventListener("resize", function () {
    window.clearTimeout(timer);
    timer = window.setTimeout(huvFitInlineMath, 200);
  });
})();
