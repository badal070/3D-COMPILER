import * as THREE from "https://esm.sh/three@0.160.1";
import { OrbitControls } from "https://esm.sh/three@0.160.1/examples/jsm/controls/OrbitControls";
import { DragControls } from "https://esm.sh/three@0.160.1/examples/jsm/controls/DragControls";
import { TransformControls } from "https://esm.sh/three@0.160.1/examples/jsm/controls/TransformControls";
import { CSS2DObject, CSS2DRenderer } from "https://esm.sh/three@0.160.1/examples/jsm/renderers/CSS2DRenderer";

export class SceneManager {
  constructor(canvas) {
    this.canvas = canvas;
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0xf2e9d8);

    this.camera = new THREE.PerspectiveCamera(55, 1, 0.1, 2000);
    this.camera.position.set(12, 10, 12);

    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    this.labelRenderer = new CSS2DRenderer();
    this.labelRenderer.domElement.style.position = "absolute";
    this.labelRenderer.domElement.style.top = "0";
    this.labelRenderer.domElement.style.left = "0";
    this.labelRenderer.domElement.style.pointerEvents = "none";
    canvas.parentElement.appendChild(this.labelRenderer.domElement);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.enablePan = true;
    this.controls.enableZoom = true;
    this.controls.screenSpacePanning = true;
    this.controls.rotateSpeed = 0.85;
    this.controls.zoomSpeed = 1.1;
    this.controls.panSpeed = 0.9;
    this.controls.minDistance = 0.5;
    this.controls.maxDistance = 500;
    this.controls.mouseButtons = {
      LEFT: THREE.MOUSE.ROTATE,
      MIDDLE: THREE.MOUSE.DOLLY,
      RIGHT: THREE.MOUSE.PAN,
    };
    this.controls.touches = {
      ONE: THREE.TOUCH.ROTATE,
      TWO: THREE.TOUCH.DOLLY_PAN,
    };

    this.transformControls = new TransformControls(this.camera, this.renderer.domElement);
    this.transformControls.visible = false;
    this.transformControls.setMode("translate");
    this.transformControls.setSpace("local");
    this.isTransformDragging = false;
    this.transformControls.addEventListener("dragging-changed", (event) => {
      this.isTransformDragging = !!event.value;
      this.controls.enabled = !event.value && !this.dragModeEnabled;
    });
    this.scene.add(this.transformControls);

    this.grid = new THREE.GridHelper(200, 200, 0x9f9581, 0xc6bca8);
    this.scene.add(this.grid);

    this.axes = new THREE.AxesHelper(3.5);
    this.axes.position.set(-8, 0, -8);
    this.scene.add(this.axes);

    this.scene.add(new THREE.AmbientLight(0xffffff, 0.55));
    const key = new THREE.DirectionalLight(0xffffff, 0.85);
    key.position.set(5, 11, 7);
    this.scene.add(key);

    this.objectMap = new Map();
    this.dragObjects = [];
    this.annotations = [];
    this.selectedId = null;
    this.csg = null;
    this.snapEnabled = true;
    this.measureMode = false;
    this.dragModeEnabled = false;
    this.measurePoints = [];

    this.raycaster = new THREE.Raycaster();
    this.pointer = new THREE.Vector2();
    this.onSelect = () => {};
    this.onTransform = () => {};

    this.dragControls = new DragControls(this.dragObjects, this.camera, this.renderer.domElement);
    this.dragControls.enabled = false;
    this.dragControls.addEventListener("hoveron", () => {
      if (this.dragModeEnabled) this.canvas.style.cursor = "grab";
    });
    this.dragControls.addEventListener("hoveroff", () => {
      if (this.dragModeEnabled) this.canvas.style.cursor = "grab";
    });
    this.dragControls.addEventListener("dragstart", (event) => {
      if (!this.dragModeEnabled) return;
      this.controls.enabled = false;
      this.transformControls.enabled = false;
      this.canvas.style.cursor = "grabbing";
      const id = event.object?.userData?.entityId;
      if (id) this.selectObject(String(id), true);
    });
    this.dragControls.addEventListener("dragend", (event) => {
      if (!this.dragModeEnabled) return;
      this.controls.enabled = true;
      this.transformControls.enabled = true;
      this.canvas.style.cursor = "grab";
      this.emitTransformForObject(event.object);
    });

    this.canvas.addEventListener("pointerdown", (event) => this.onPointerDown(event));
    this.transformControls.addEventListener("objectChange", () => this.emitTransform());
    this.setSnapEnabled(this.snapEnabled);

    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(this.canvas.parentElement);
    this.resize();
    this.loadCsg();
    this.animate();
  }

  async loadCsg() {
    try {
      this.csg = await import("https://esm.sh/three-bvh-csg@0.0.16?deps=three@0.160.1");
    } catch {
      this.csg = null;
    }
  }

  setCallbacks({ onSelect, onTransform } = {}) {
    if (onSelect) this.onSelect = onSelect;
    if (onTransform) this.onTransform = onTransform;
  }

  setSnapEnabled(enabled) {
    this.snapEnabled = enabled;
    this.transformControls.setTranslationSnap(enabled ? 0.5 : null);
    this.transformControls.setRotationSnap(enabled ? THREE.MathUtils.degToRad(15) : null);
    this.transformControls.setScaleSnap(enabled ? 0.1 : null);
  }

  setMeasureMode(enabled) {
    this.measureMode = enabled;
    this.measurePoints = [];
  }

  setTransformMode(mode) {
    const nextMode = String(mode || "").toLowerCase();
    if (!["translate", "rotate", "scale"].includes(nextMode)) return;
    this.transformControls.setMode(nextMode);
  }

  setDragMode(enabled) {
    this.dragModeEnabled = !!enabled;
    this.dragControls.enabled = this.dragModeEnabled;
    this.controls.enabled = !this.isTransformDragging;

    const selected = this.selectedId ? this.objectMap.get(this.selectedId) : null;
    if (selected && !this.dragModeEnabled) {
      this.transformControls.attach(selected);
      this.transformControls.visible = true;
    } else {
      this.transformControls.detach();
      this.transformControls.visible = false;
    }

    this.canvas.style.cursor = this.dragModeEnabled ? "grab" : "default";
  }

  updateScene(ir) {
    const entities = Array.isArray(ir?.entities) ? ir.entities : [];
    const incoming = new Set(entities.map((entity) => String(entity.id)));

    for (const [id, mesh] of this.objectMap.entries()) {
      if (!incoming.has(id)) {
        this.scene.remove(mesh);
        mesh.geometry?.dispose();
        mesh.material?.dispose();
        this.objectMap.delete(id);
      }
    }

    const precision = Number(ir?.metadata?.precision ?? 1);
    this.updateGrid(precision > 0 ? precision : 1);

    for (const entity of entities) {
      const id = String(entity.id);
      const transform = getComponent(entity, "transform");
      const geometryComponent = getComponent(entity, "geometry") || getComponent(entity, "solid");

      const primitive = String(getProperty(geometryComponent, "primitive", "box")).toLowerCase();
      const dimensions = toVector3(getProperty(geometryComponent, "dimensions", [1, 1, 1]));
      const position = toVector3(getProperty(transform, "position", [0, 0, 0]));
      const rotation = toVector3(getProperty(transform, "rotation", [0, 0, 0]));
      const scale = toVector3(getProperty(transform, "scale", [1, 1, 1]));

      let mesh = this.objectMap.get(id);
      if (!mesh) {
        mesh = new THREE.Mesh(
          createGeometry(primitive, dimensions),
          new THREE.MeshStandardMaterial({ color: 0xc47a4e, roughness: 0.48, metalness: 0.06 }),
        );
        mesh.userData.entityId = id;
        this.scene.add(mesh);
        this.objectMap.set(id, mesh);
      } else {
        const prevSig = mesh.userData.geometrySignature;
        const nextSig = `${primitive}:${dimensions.join(":")}`;
        if (prevSig !== nextSig) {
          mesh.geometry.dispose();
          mesh.geometry = createGeometry(primitive, dimensions);
        }
      }

      mesh.userData.entity = entity;
      mesh.userData.geometrySignature = `${primitive}:${dimensions.join(":")}`;
      mesh.position.set(position[0], position[1], position[2]);
      mesh.rotation.set(rotation[0], rotation[1], rotation[2]);
      mesh.scale.set(scale[0], scale[1], scale[2]);

      this.setDimensionLabel(mesh, dimensions);
    }

    this.updateDragObjects();
    this.applyBooleanConstraints(Array.isArray(ir?.constraints) ? ir.constraints : []);

    if (this.selectedId) {
      this.selectObject(this.selectedId, false);
    }
  }

  selectObject(id, emit = true) {
    for (const [meshId, mesh] of this.objectMap.entries()) {
      const material = mesh.material;
      if (material && "emissive" in material) {
        material.emissive.set(meshId === id ? 0x5e2a13 : 0x000000);
      }
    }

    const selected = this.objectMap.get(id);
    this.selectedId = selected ? id : null;

    if (selected) {
      if (!this.dragModeEnabled) {
        this.transformControls.attach(selected);
        this.transformControls.visible = true;
      } else {
        this.transformControls.detach();
        this.transformControls.visible = false;
      }
      if (emit) this.onSelect(selected.userData.entity);
      return;
    }

    this.transformControls.detach();
    this.transformControls.visible = false;
    if (emit) this.onSelect(null);
  }

  focusObject(id) {
    const mesh = this.objectMap.get(String(id));
    if (!mesh) return;
    this.selectObject(String(id), true);
    const target = mesh.position.clone();
    this.controls.target.copy(target);
    this.camera.position.set(target.x + 8, target.y + 7, target.z + 8);
    this.controls.update();
  }

  emitTransform() {
    this.emitTransformForObject(this.transformControls.object);
  }

  emitTransformForObject(object) {
    if (!object?.userData?.entity) return;
    const entity = structuredClone(object.userData.entity);
    const transform = getComponent(entity, "transform");
    if (!transform.properties) transform.properties = {};
    transform.properties.position = { Vector3: [object.position.x, object.position.y, object.position.z] };
    transform.properties.rotation = { Vector3: [object.rotation.x, object.rotation.y, object.rotation.z] };
    transform.properties.scale = { Vector3: [object.scale.x, object.scale.y, object.scale.z] };
    object.userData.entity = entity;
    this.onTransform(entity);
  }

  onPointerDown(event) {
    if (this.isTransformDragging) return;
    const rect = this.canvas.getBoundingClientRect();
    this.pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    this.pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

    this.raycaster.setFromCamera(this.pointer, this.camera);
    const intersects = this.raycaster.intersectObjects([...this.objectMap.values()], false);
    if (!intersects.length) return;

    const hit = intersects[0];
    const id = hit.object.userData.entityId;

    if (this.measureMode) {
      this.measurePoints.push(hit.point.clone());
      if (this.measurePoints.length === 2) {
        this.addMeasurement(this.measurePoints[0], this.measurePoints[1]);
        this.measurePoints = [];
      }
      return;
    }

    if (id) this.selectObject(String(id), true);
  }

  addMeasurement(start, end) {
    const geometry = new THREE.BufferGeometry().setFromPoints([start, end]);
    const line = new THREE.Line(
      geometry,
      new THREE.LineBasicMaterial({ color: 0x227948, linewidth: 1 }),
    );
    this.scene.add(line);

    const center = start.clone().add(end).multiplyScalar(0.5);
    const distance = start.distanceTo(end);
    const label = document.createElement("div");
    label.style.padding = "2px 6px";
    label.style.border = "1px solid rgba(34,121,72,0.35)";
    label.style.borderRadius = "999px";
    label.style.background = "rgba(255,255,255,0.9)";
    label.style.color = "#20593b";
    label.style.fontFamily = "IBM Plex Mono, monospace";
    label.style.fontSize = "11px";
    label.textContent = `${distance.toFixed(3)}`;

    const tag = new CSS2DObject(label);
    tag.position.copy(center);
    this.scene.add(tag);

    this.annotations.push(line, tag);
  }

  setDimensionLabel(mesh, dims) {
    if (mesh.userData.dimensionTag) {
      mesh.remove(mesh.userData.dimensionTag);
    }

    const label = document.createElement("div");
    label.style.fontFamily = "IBM Plex Mono, monospace";
    label.style.fontSize = "10px";
    label.style.padding = "2px 5px";
    label.style.borderRadius = "5px";
    label.style.background = "rgba(255,255,255,0.82)";
    label.style.border = "1px solid rgba(60,50,36,0.16)";
    label.textContent = `${dims.map((n) => Number(n).toFixed(2)).join(" × ")}`;

    const tag = new CSS2DObject(label);
    tag.position.set(0, Math.max(...dims) * 0.55, 0);
    mesh.add(tag);
    mesh.userData.dimensionTag = tag;
  }

  clearMeasurements() {
    for (const item of this.annotations) {
      if (item.parent) item.parent.remove(item);
      if (item.geometry) item.geometry.dispose();
      if (item.material) item.material.dispose();
    }
    this.annotations = [];
    this.measurePoints = [];
  }

  updateDragObjects() {
    this.dragObjects.length = 0;
    this.dragObjects.push(...this.objectMap.values());
  }

  applyBooleanConstraints(constraints) {
    if (!this.csg) return;
    const { Evaluator, Brush, SUBTRACTION, ADDITION, INTERSECTION } = this.csg;
    if (!Evaluator || !Brush) return;

    const evaluator = new Evaluator();
    for (const constraint of constraints) {
      const type = String(constraint?.constraint_type || "").toLowerCase();
      if (!["boolean_subtract", "boolean_union", "boolean_intersect"].includes(type)) continue;

      const targetId = String(unwrapIrValue(constraint?.parameters?.target || ""));
      const toolId = String(unwrapIrValue(constraint?.parameters?.tool || ""));
      const targetMesh = this.objectMap.get(targetId);
      const toolMesh = this.objectMap.get(toolId);
      if (!targetMesh || !toolMesh) continue;

      try {
        const targetBrush = new Brush(targetMesh.geometry.clone());
        targetBrush.position.copy(targetMesh.position);
        targetBrush.rotation.copy(targetMesh.rotation);
        targetBrush.scale.copy(targetMesh.scale);
        targetBrush.updateMatrixWorld(true);

        const toolBrush = new Brush(toolMesh.geometry.clone());
        toolBrush.position.copy(toolMesh.position);
        toolBrush.rotation.copy(toolMesh.rotation);
        toolBrush.scale.copy(toolMesh.scale);
        toolBrush.updateMatrixWorld(true);

        const operation =
          type === "boolean_subtract"
            ? SUBTRACTION
            : type === "boolean_union"
              ? ADDITION
              : INTERSECTION;

        const result = evaluator.evaluate(targetBrush, toolBrush, operation);
        if (result?.geometry) {
          targetMesh.geometry.dispose();
          targetMesh.geometry = result.geometry;
        }
      } catch {
        // Keep base geometry if CSG preview fails.
      }
    }
  }

  updateGrid(precision) {
    const spacing = Math.max(precision, 0.01);
    const divisions = Math.min(500, Math.max(20, Math.round(20 / spacing)));
    this.scene.remove(this.grid);
    this.grid.geometry.dispose();
    this.grid.material.dispose();
    this.grid = new THREE.GridHelper(200, divisions, 0x9f9581, 0xc6bca8);
    this.scene.add(this.grid);
  }

  resize() {
    const { clientWidth, clientHeight } = this.canvas.parentElement;
    if (!clientWidth || !clientHeight) return;
    this.camera.aspect = clientWidth / clientHeight;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(clientWidth, clientHeight, false);
    this.labelRenderer.setSize(clientWidth, clientHeight);
  }

  animate() {
    requestAnimationFrame(() => this.animate());
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
    this.labelRenderer.render(this.scene, this.camera);
  }
}

function createGeometry(primitive, dimensions) {
  const [x, y, z] = dimensions;
  switch (primitive) {
    case "sphere": {
      const radius = Math.max(x, y, z) * 0.5;
      return new THREE.SphereGeometry(radius, 28, 20);
    }
    case "cylinder": {
      const radius = Math.max(x, z) * 0.5;
      return new THREE.CylinderGeometry(radius, radius, Math.max(y, 0.001), 28);
    }
    case "cone": {
      const radius = Math.max(x, z) * 0.5;
      return new THREE.ConeGeometry(radius, Math.max(y, 0.001), 28);
    }
    case "torus": {
      const major = Math.max(x, 0.05) * 0.5;
      const minor = Math.max(y, 0.05) * 0.25;
      return new THREE.TorusGeometry(major, minor, 20, 64);
    }
    case "plane": {
      return new THREE.PlaneGeometry(Math.max(x, 0.001), Math.max(y, 0.001));
    }
    default:
      return new THREE.BoxGeometry(Math.max(x, 0.001), Math.max(y, 0.001), Math.max(z, 0.001));
  }
}

function getComponent(entity, name) {
  const components = entity?.components;
  if (!components || typeof components !== "object") return {};
  return components[name] || {};
}

function getProperty(component, key, fallback = null) {
  const props = component?.properties;
  if (!props || typeof props !== "object") return fallback;
  if (!(key in props)) return fallback;
  return unwrapIrValue(props[key]);
}

function unwrapIrValue(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return value;
  const keys = Object.keys(value);
  if (keys.length !== 1) return value;
  const type = keys[0];
  if (["Number", "String", "Identifier", "Boolean", "Vector3", "Matrix3"].includes(type)) {
    return value[type];
  }
  if (type === "List" && Array.isArray(value[type])) {
    return value[type].map((item) => unwrapIrValue(item));
  }
  return value;
}

function toVector3(value) {
  if (Array.isArray(value) && value.length >= 3) {
    return [Number(value[0]), Number(value[1]), Number(value[2])];
  }
  if (typeof value === "number") {
    return [value, value, value];
  }
  return [0, 0, 0];
}
