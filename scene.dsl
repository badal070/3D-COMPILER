scene {
  name: "Two Cubes Self Rotation"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
  math: "core_mechanics"
  geometry: "basic_solids"
}

entity cube_a {
  kind: solid
  components {
    transform {
      position: [-3, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: cube
    }
  }
}

entity cube_b {
  kind: solid
  components {
    transform {
      position: [3, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: cube
    }
  }
}

motion rotate_cube_a {
  target: cube_a
  type: rotation
  axis: [0, 1, 0]
  speed: 1.0
}

motion rotate_cube_b {
  target: cube_b
  type: rotation
  axis: [1, 0, 0]
  speed: 1.5
}

timeline main {
  event {
    motion: rotate_cube_a
    start: 0.0
    duration: 10.0
  }
  event {
    motion: rotate_cube_b
    start: 0.0
    duration: 10.0
  }
}
