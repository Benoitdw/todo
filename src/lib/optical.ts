/**
 * OPTICAL ALIGNMENT — put display type's INK on the column line, not its box.
 *
 * A 64px headline whose layout box is exactly on line 1 still reads as
 * indented against 16px body text, because the letterform's ink is inset by
 * its left side-bearing. We measure that side-bearing on the ACTUALLY LOADED
 * font and shift the box by it, so the visible ink lands on the line.
 *
 * Side-bearing is font-specific, so the measurement is redone after
 * document.fonts.ready (the webfont is self-hosted and bundled, see main.ts),
 * whenever the text changes, and on resize — the display sizes are
 * breakpoint-dependent.
 *
 * Usage:  <h1 class="masthead" use:optical={list.title}>{list.title}</h1>
 *         (the argument is only a change token: pass whatever text is rendered)
 */

let ctx: CanvasRenderingContext2D | null | undefined;

function measuringContext(): CanvasRenderingContext2D | null {
  if (ctx === undefined) ctx = document.createElement('canvas').getContext('2d');
  return ctx;
}

export function optical(node: HTMLElement, _token?: unknown) {
  let frame = 0;

  function measure() {
    node.style.marginLeft = '0px'; // measure from the true box, never a shifted one

    const text = (node.textContent ?? '').trim();
    if (!text) return;

    const cs = getComputedStyle(node);
    let ch = text.charAt(0);
    if (cs.textTransform === 'uppercase') ch = ch.toUpperCase();

    const c = measuringContext();
    if (!c) return;

    c.font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
    c.textAlign = 'left';

    // +ve = the ink overhangs to the LEFT of the box origin
    const bearing = c.measureText(ch).actualBoundingBoxLeft;
    if (Number.isFinite(bearing)) node.style.marginLeft = `${bearing.toFixed(2)}px`;
  }

  function schedule() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(measure);
  }

  schedule();
  document.fonts?.ready.then(schedule);
  window.addEventListener('resize', schedule);

  return {
    update: schedule,
    destroy() {
      cancelAnimationFrame(frame);
      window.removeEventListener('resize', schedule);
    },
  };
}
