import init, { go_to_pvl_hub, go_to_home } from './pkg/bevy_chess.js';

const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
let isPaused = false;

window.show_game_controls = () => {
  if (isTouchDevice) {
    document.getElementById('menu-btn').style.display = 'block';
  } else {
    document.getElementById('esc-hint').style.display = 'block';
  }
};
window.hide_game_controls = () => {
  document.getElementById('esc-hint').style.display = 'none';
  document.getElementById('menu-btn').style.display = 'none';
  // Close pause overlay if open (state transition happened — game is over or exited)
  if (isPaused) {
    isPaused = false;
    document.getElementById('pause-overlay').style.display = 'none';
    document.getElementById('canvas').style.pointerEvents = 'auto';
  }
};

window.set_pvl_mode = function(isPvL) {
  const hubBtn = document.getElementById('pause-hub-btn');
  if (hubBtn) hubBtn.style.display = isPvL ? 'block' : 'none';
};

function fitCanvas() {
  const canvas = document.getElementById('canvas');
  if (!canvas.offsetWidth) return;
  const scaleX = window.innerWidth  / canvas.offsetWidth;
  const scaleY = window.innerHeight / canvas.offsetHeight;
  let factor = Math.min(scaleX, scaleY);
  if (!isTouchDevice) factor = Math.min(factor, 1.0); // desktop: don't upscale
  canvas.style.transform = `translate(-50%, -50%) scale(${factor})`;
}

function closePause() {
  isPaused = false;
  document.getElementById('pause-overlay').style.display = 'none';
  document.getElementById('canvas').style.pointerEvents = 'auto';
}

window.togglePause = function () {
  isPaused = !isPaused;
  const overlay = document.getElementById('pause-overlay');
  const canvas  = document.getElementById('canvas');

  if (isPaused) {
    overlay.style.display = 'flex';
    canvas.style.pointerEvents = 'none';
  } else {
    overlay.style.display = 'none';
    canvas.style.pointerEvents = 'auto';
  }
};

window.pauseGoToHub = function() {
  closePause();
  go_to_pvl_hub();
};

window.pauseGoToHome = function() {
  closePause();
  go_to_home();
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
  fitCanvas();
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
