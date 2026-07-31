/**
 * Fit a fixed 1920×1080 stage into the browser viewport (letterboxed, no scroll).
 * Usage:
 *   <div id="stage" data-fit-w="1920" data-fit-h="1080">...</div>
 *   <script src="../assets/viewport-fit.js"></script>
 */
(function () {
  var STAGE_ID = 'stage';
  var DEFAULT_W = 1920;
  var DEFAULT_H = 1080;

  function fit() {
    var stage = document.getElementById(STAGE_ID);
    if (!stage) return;
    var w = Number(stage.dataset.fitW) || DEFAULT_W;
    var h = Number(stage.dataset.fitH) || DEFAULT_H;
    var s = Math.min(window.innerWidth / w, window.innerHeight / h);
    stage.style.width = w + 'px';
    stage.style.height = h + 'px';
    stage.style.transform = 'scale(' + s + ')';
    stage.style.transformOrigin = 'center center';
  }

  window.addEventListener('resize', fit);
  window.addEventListener('load', fit);
  fit();
})();
