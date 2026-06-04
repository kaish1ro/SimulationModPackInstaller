/* global React, CMD_BLOCK */

/* ============================================================
   COMMAND BLOCK — 3D CSS cube with real Minecraft textures
   Repeating (purple) command block, isometric rotation
   ============================================================ */
function CommandBlock({ size = 80 }) {
  const s = size;
  const half = s / 2;

  const face = (url) => ({
    position: 'absolute',
    width: s, height: s,
    backgroundImage: `url("${url}")`,
    backgroundSize: 'cover',
    imageRendering: 'pixelated',
  });

  return (
    <div style={{
      width: s, height: s,
      position: 'relative',
      transformStyle: 'preserve-3d',
      animation: 'cbSpin 8s linear infinite',
    }}>
      {/* Front */}
      <div style={{ ...face(CMD_BLOCK.front),
        transform: `translateZ(${half}px)` }} />
      {/* Back */}
      <div style={{ ...face(CMD_BLOCK.back),
        transform: `rotateY(180deg) translateZ(${half}px)` }} />
      {/* Left */}
      <div style={{ ...face(CMD_BLOCK.side),
        transform: `rotateY(-90deg) translateZ(${half}px)` }} />
      {/* Right */}
      <div style={{ ...face(CMD_BLOCK.side),
        transform: `rotateY(90deg) translateZ(${half}px)` }} />
      {/* Top */}
      <div style={{ ...face(CMD_BLOCK.side),
        transform: `rotateX(90deg) translateZ(${half}px)` }} />
      {/* Bottom */}
      <div style={{ ...face(CMD_BLOCK.side),
        transform: `rotateX(-90deg) translateZ(${half}px)` }} />

      <style>{`
        @keyframes cbSpin {
          0%   { transform: rotateX(-20deg) rotateY(0deg); }
          100% { transform: rotateX(-20deg) rotateY(360deg); }
        }
      `}</style>
    </div>
  );
}

Object.assign(window, { CommandBlock });
