scene {
  name: "Helical Spheres"
  version: 2
  ir_version: "0.2.0"
  unit_system: "SI"
}

entities
entity sphere_a {
  kind: solid
  components {
    transform { position: [0.0, 0.0, 0.0] }
    geometry { primitive: sphere }
  }
}
entity sphere_b {
  kind: solid
  components {
    transform { position: [0.2, 0.0, 0.0] }
    geometry { primitive: sphere }
  }
}

trajectories
trajectory helix_a {
  target: sphere_a
  path_type: helix
  axis: [0.0, 1.0, 0.0]
  center: [0.0, 0.0, 0.0]
  radius: 0.6
  pitch: 0.4
  turns: 6
  start_angle: 0.0
}
trajectory helix_b {
  target: sphere_b
  path_type: helix
  axis: [0.0, 1.0, 0.0]
  center: [0.0, 0.0, 0.0]
  radius: 0.4
  pitch: 0.35
  turns: 6
  start_angle: 0.8
}

timeline helix_play {
  event {
    trajectory: helix_a
    start: 0.0
    duration: 12.0
  }
  event {
    trajectory: helix_b
    start: 0.0
    duration: 12.0
  }
}