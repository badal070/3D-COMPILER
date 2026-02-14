scene {
  name: "Two Spheres Helical Motion"
  version: 1
  ir_version: "0.1.0"
  unit_system: "SI"
}

library_imports {
  math: "core_mechanics"
  geometry: "basic_solids"
}

entity sphere_a {
  kind: solid
  components {
    transform {
      position: [0, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: sphere
    }
  }
}

entity sphere_b {
  kind: solid
  components {
    transform {
      position: [0, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: sphere
    }
  }
}

// Base motions: rotation around Z and translation along Z
motion spin_a {
  target: sphere_a
  type: rotation
  axis: [0, 0, 1]
  speed: 2.0
}

motion lift_a {
  target: sphere_a
  type: translation
  direction: [0, 0, 1]
  speed: 0.5
}

motion spin_b {
  target: sphere_b
  type: rotation
  axis: [0, 0, 1]
  speed: 2.0
}

motion lift_b {
  target: sphere_b
  type: translation
  direction: [0, 0, 1]
  speed: 0.5
}

// Compound motions use the validated DSL feature and are lowered to IR
compound_motion helix_a {
  type: parallel
  motions: "spin_a,lift_a"
}

compound_motion helix_b {
  type: parallel
  motions: "spin_b,lift_b"
}

timeline main {
  event {
    motion: spin_a
    start: 0.0
    duration: 10.0
  }
  event {
    motion: lift_a
    start: 0.0
    duration: 10.0
  }
  event {
    motion: spin_b
    start: 0.0
    duration: 10.0
  }
  event {
    motion: lift_b
    start: 0.0
    duration: 10.0
  }
}

