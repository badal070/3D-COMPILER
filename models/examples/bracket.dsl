scene {
  name: "bracket_assembly_v3"
  version: 3
  ir_version: "2.0.0"
  unit_system: SI
  domain: modeling
  precision: 0.01
  author: "user@example.com"
  created: "2026-02-26"
}

library_imports {
  modeling: "modeling_core"
}

materials {
  material steel {
    density: 1.0
    elasticity: 0.5
    friction: 0.3
  }
}

entity body {
  kind: solid
  components {
    transform {
      position: [0, 0, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: box
      dimensions: [50, 30, 10]
    }
    solid {
      primitive: box
      dimensions: [50, 30, 10]
    }
    material_ref {
      name: steel
    }
  }
}

entity hole_fl {
  kind: solid
  components {
    transform {
      position: [-18, -10, 0]
      rotation: [0, 0, 0]
      scale: [1, 1, 1]
    }
    geometry {
      primitive: cylinder
      dimensions: [6.2, 10, 6.2]
    }
    solid {
      primitive: cylinder
      dimensions: [6.2, 10, 6.2]
    }
  }
}

entity top_edge_fillet {
  kind: feature
  components {
    fillet {
      target: body
      edges: [body_top_front, body_top_back]
      radius: 4
    }
  }
}

constraint hole_fl_subtract {
  type: boolean_subtract
  target: body
  tool: hole_fl
}

timeline modeling_main {
}
