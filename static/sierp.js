function lerp(value1, value2, amount) {
  amount = amount < 0 ? 0 : amount;
  amount = amount > 1 ? 1 : amount;
  return value1 + (value2 - value1) * amount;
}

function computeSierp(points) {
  const firstTri = [0, 0.5, 0, 0.5, 0, 0.5, -0.5, 0, 0.5, 0, 0, -0.5];

  let randomPoint = Math.floor(Math.random() * 4);
  let lastPoint = {
    x: firstTri[randomPoint * 3],
    y: firstTri[randomPoint * 3 + 1],
    z: firstTri[randomPoint * 3 + 2]
  };
  points[0] = lastPoint.x;
  points[1] = lastPoint.y;
  points[2] = lastPoint.z;

  for (let i = 0; i < points.length; i += 3) {
    randomPoint = Math.floor(Math.random() * 4);
    let newPoint = {
      x: lerp(firstTri[randomPoint * 3], lastPoint.x, 0.5),
      y: lerp(firstTri[randomPoint * 3 + 1], lastPoint.y, 0.5),
      z: lerp(firstTri[randomPoint * 3 + 2], lastPoint.z, 0.5)
    };
    lastPoint = newPoint;
    points[i] = newPoint.x;
    points[i + 1] = newPoint.y;
    points[i + 2] = newPoint.z;
  }
}
