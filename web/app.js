import init from './pkg/bevy_chess.js';

const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
let isLandscape = window.innerWidth > window.innerHeight;
let isPaused = false;

function fitCanvas() {
  const canvas = document.getElementById('canvas');
  if (!canvas.offsetWidth) return;
  const scaleX = window.innerWidth  / canvas.offsetWidth;
  const scaleY = window.innerHeight / canvas.offsetHeight;
  let factor = Math.min(scaleX, scaleY);
  if (!isTouchDevice) factor = Math.min(factor, 1.0); // desktop: don't upscale
  canvas.style.transform = `translate(-50%, -50%) scale(${factor})`;
}

function updatePortraitHint() {
  const nowLandscape = window.innerWidth > window.innerHeight;
  const hint = document.getElementById('portrait-hint');
  if (isTouchDevice && !nowLandscape) {
    hint.style.display = 'flex';
  } else {
    hint.style.display = 'none';
  }
}

window.togglePause = function () {
  isPaused = !isPaused;
  const overlay = document.getElementById('pause-overlay');
  const canvas  = document.getElementById('canvas');
  const btn     = document.getElementById('btn-pause');

  if (isPaused) {
    overlay.style.display = 'flex';
    canvas.style.pointerEvents = 'none';
    btn.textContent = '▶ Reanudar';
  } else {
    overlay.style.display = 'none';
    canvas.style.pointerEvents = 'auto';
    btn.textContent = '⏸ Pausa';
  }
};

window.exitGame = function () {
  if (confirm('¿Empezar una nueva partida?')) {
    window.location.reload();
  }
};

// Intercept Escape before Bevy's canvas handler can swallow it
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    window.togglePause();
    e.preventDefault();
    e.stopPropagation();
  }
}, true); // capture phase — fires before canvas receives the event

window.addEventListener('resize', () => {
  isLandscape = window.innerWidth > window.innerHeight;
  fitCanvas();
  updatePortraitHint();
});

// Boot
// wasm-bindgen throws a control-flow exception to exit the Bevy loop setup — not a real error
try {
  await init();
} catch (e) {
  if (!(e instanceof Error) || !e.message.includes('control flow')) throw e;
}
document.getElementById('loading-overlay').style.display = 'none';
fitCanvas();
updatePortraitHint();
