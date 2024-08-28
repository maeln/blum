const gl = document
  .getElementById("c")
  .getContext("webgl", { preserveDrawingBuffer: true });
twgl.addExtensionsToContext(gl);
const m4 = twgl.m4;
gl.clearColor(0, 0, 0, 1.0);

const programInfo = twgl.createProgramInfo(gl, ["sierp-vs", "sierp-fs"]);

let points = new Array(2048);
computeSierp(points);

const arrays = {
  position: {
    numComponents: 3,
    data: points,
  },
};
const bufferInfo = twgl.createBufferInfoFromArrays(gl, arrays);

let dt = 0;
let lt = 0;
let nt = 0;
let sw = true;

let slide = 0;

let pz = 0;
let py = 0;
let px = 0;

let pax = 0.0;
let pay = 0.0;
let paz = 0.0;

const speed = 0.00001;
const attractorPower = 3.0;
const attractorSpeed = 0.00005;

const rx = (2.0 * Math.random() - 1) * 0.0001;
const ry = (2.0 * Math.random() - 1) * 0.0001;
const rz = (2.0 * Math.random() - 1) * 0.0001;
const timeAdvance = Math.random() * 10.0;

/*
let attractorWeight = 50;
let sierpWeight = 80;

const calc_f = (d) => {
  const G = 6.7 * Math.pow(10, -11);
  return (G * attractorWeight * sierpWeight) / Math.pow(d, 2);
}
*/

let f = 0;

let vz = (paz - pz) * attractorPower;
let vy = (pay - py) * attractorPower;
let vx = (pax - px) * attractorPower;

gl.enable(gl.DEPTH_TEST);
gl.enable(gl.CULL_FACE);
gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

function resize() {
  twgl.resizeCanvasToDisplaySize(gl.canvas);
  gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
}
resize();

window.onresize = resize;

let ft = 0;

function render(time) {
  // gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  dt = time - lt;
  lt = time;

  // if we spent more than 300ms, we assume that it was
  // because the user left the screen and we skip this frame.
  if (dt > 300) {
    // console.log("Skipping ", dt);
  } else {
    pax = Math.cos(5.0 * (time + timeAdvance) * attractorSpeed) * 2.0;
    pay = Math.sin(6.0 * (time + timeAdvance) * attractorSpeed) * 2.0;
    paz = Math.cos(7.0 * (time + timeAdvance) * attractorSpeed) * 2.0;

    ft += 1;
    if (nt >= Math.sqrt(1.0) && sw) {
      sw = false;
    }
    if (nt < 0) {
      sw = true;
    }

    if (!sw) {
      nt -= dt * 0.0002;
    } else {
      nt += dt * 0.0002;
    }
    const uniforms = {
      time: nt,
      resolution: [gl.canvas.width, gl.canvas.height],
    };

    const ratio = gl.canvas.width / gl.canvas.height;
    const perspective = m4.perspective(
      45.0 * (3.1415 / 180.0),
      ratio,
      0.001,
      100.0
    );
    const lookAt = m4.inverse(
      m4.lookAt([0.0, 0.0, 2.0], [0.0, 0.0, 0.0], [0, 1, 0])
    );

    vz = (paz - pz) * attractorPower;
    vy = (pay - py) * attractorPower;
    vx = (pax - px) * attractorPower;

    py += vy * dt * speed;
    pz += vz * dt * speed;
    px += vx * dt * speed;

    if (px >= 0.9 || px <= -0.9) {
      vx = -vx;
    }

    const world = m4.multiply(
      m4.translation(twgl.v3.create(px, py, pz)),
      m4.multiply(
        m4.rotationZ(time * rz),
        m4.multiply(m4.rotationY(time * ry), m4.rotationX(time * rx))
      )
    );
    //const world = m4.translation(twgl.v3.create(px, py, pz));

    uniforms.u_world = world;
    uniforms.u_view = lookAt;
    uniforms.u_perspective = perspective;

    gl.useProgram(programInfo.program);
    twgl.setBuffersAndAttributes(gl, programInfo, bufferInfo);
    twgl.setUniforms(programInfo, uniforms);
    twgl.drawBufferInfo(gl, bufferInfo, gl.POINTS);
  }

  requestAnimationFrame(render);
}

requestAnimationFrame(render);
