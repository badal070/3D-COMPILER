scene {
  name: "Nested Circular Cubes"
  version: 2
  ir_version: "0.2.0"
  unit_system: "SI"
}

entities
entity cube1 { kind: solid components { transform { position: [1.5, 0.0, 0.0] } geometry { primitive: box } } }
entity cube2 { kind: solid components { transform { position: [0.5, 0.0, 0.0] } geometry { primitive: box } } }
entity cube3 { kind: solid components { transform { position: [-0.5, 0.0, 0.0] } geometry { primitive: box } } }
entity cube4 { kind: solid components { transform { position: [-1.5, 0.0, 0.0] } geometry { primitive: box } } }

motions
# inner pair orbits (each pair revolves about its local pair center)
motion inner_m1 { target: cube1 type: orbital center: [1.0, 0.0, 0.0] radius: 0.5 normal: [0.0,1.0,0.0] speed: 1.0 }
motion inner_m2 { target: cube2 type: orbital center: [1.0, 0.0, 0.0] radius: 0.5 normal: [0.0,1.0,0.0] speed: 1.0 }
motion inner_m3 { target: cube3 type: orbital center: [-1.0, 0.0, 0.0] radius: 0.5 normal: [0.0,1.0,0.0] speed: 1.0 }
motion inner_m4 { target: cube4 type: orbital center: [-1.0, 0.0, 0.0] radius: 0.5 normal: [0.0,1.0,0.0] speed: 1.0 }

# outer group orbits (each cube also participates in a larger group orbit around scene center)
motion outer_m1 { target: cube1 type: orbital center: [0.0, 0.0, 0.0] radius: 2.0 normal: [0.0,1.0,0.0] speed: 0.4 }
motion outer_m2 { target: cube2 type: orbital center: [0.0, 0.0, 0.0] radius: 2.0 normal: [0.0,1.0,0.0] speed: 0.4 }
motion outer_m3 { target: cube3 type: orbital center: [0.0, 0.0, 0.0] radius: 2.0 normal: [0.0,1.0,0.0] speed: 0.4 }
motion outer_m4 { target: cube4 type: orbital center: [0.0, 0.0, 0.0] radius: 2.0 normal: [0.0,1.0,0.0] speed: 0.4 }

# compose each pair in parallel, then run all in parallel (nested)
compound_motion pair_left { type: parallel motions: [ inner_m3, inner_m4 ] }
compound_motion pair_right { type: parallel motions: [ inner_m1, inner_m2 ] }

# top-level nested motion: play inner pair rotations and outer group rotations together
compound_motion nested_group { type: parallel motions: [ pair_left, pair_right, outer_m1, outer_m2, outer_m3, outer_m4 ] }

timeline nested_play {
  event {
    compound_motion: nested_group
    start: 0.0
    duration: 30.0
  }
}


Failed to load resource: net::ERR_EMPTY_RESPONSE
[NEW] Explain Console errors by using Copilot in Edge: click
         
         to explain an error. 
        Learn more
        Don't show again