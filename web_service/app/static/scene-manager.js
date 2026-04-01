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
    this.renderer.localClippingEnabled = true;

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
    this.selectedIds = new Set();
    this.csg = null;
    this.currentPrecision = 1;
    this.measurementUnit = "units";
    this.snapEnabled = true;
    this.vertexSnapEnabled = false;
    this.edgeSnapEnabled = false;
    this.measureMode = false;
    this.dragModeEnabled = false;
    this.measurePoints = [];
    this.isApplyingSnap = false;
    this.transformSpace = "local";
    this.displayMode = "solid";
    this.sectionState = { enabled: false, axis: "y", offset: 0 };
    this.sectionPlane = new THREE.Plane(new THREE.Vector3(0, -1, 0), 0);
    this.objectStateMap = new Map();

    this.raycaster = new THREE.Raycaster();
    this.pointer = new THREE.Vector2();
    this.onSelect = () => {};
    this.onSelectionChange = () => {};
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
      if (event.object?.userData?.locked) return;
      this.controls.enabled = false;
      this.transformControls.enabled = false;
      this.canvas.style.cursor = "grabbing";
      const id = event.object?.userData?.entityId;
      if (id) this.selectMultiple([String(id)], true, String(id));
    });
    this.dragControls.addEventListener("dragend", (event) => {
      if (!this.dragModeEnabled) return;
      this.controls.enabled = true;
      this.transformControls.enabled = true;
      this.canvas.style.cursor = "grab";
      this.emitTransformForObject(event.object);
    });
    this.dragControls.addEventListener("drag", (event) => {
      if (!this.dragModeEnabled) return;
      this.applySnappingForObject(event.object);
    });

    this.canvas.addEventListener("pointerdown", (event) => this.onPointerDown(event));
    this.transformControls.addEventListener("objectChange", () => this.onTransformObjectChange());
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

  setCallbacks({ onSelect, onSelectionChange, onTransform } = {}) {
    if (onSelect) this.onSelect = onSelect;
    if (onSelectionChange) this.onSelectionChange = onSelectionChange;
    if (onTransform) this.onTransform = onTransform;
  }

  setSnapEnabled(enabled) {
    this.snapEnabled = enabled;
    this.transformControls.setTranslationSnap(enabled ? this.currentPrecision : null);
    this.transformControls.setRotationSnap(enabled ? THREE.MathUtils.degToRad(15) : null);
    this.transformControls.setScaleSnap(enabled ? this.currentPrecision : null);
  }

  setVertexSnapEnabled(enabled) {
    this.vertexSnapEnabled = !!enabled;
  }

  setEdgeSnapEnabled(enabled) {
    this.edgeSnapEnabled = !!enabled;
  }

  setMeasurementUnit(unitLabel) {
    const normalized = String(unitLabel || "").trim();
    this.measurementUnit = normalized || "units";
  }

  setPrecision(precision) {
    const parsed = Number(precision);
    this.currentPrecision = Number.isFinite(parsed) && parsed > 0 ? parsed : 1;
    this.setSnapEnabled(this.snapEnabled);
    this.updateGrid(this.currentPrecision);
  }

  setMeasureMode(enabled) {
    this.measureMode = enabled;
    this.measurePoints = [];
    if (!enabled) {
      this.clearMeasurements();
    }
  }

  setTransformMode(mode) {
    const nextMode = String(mode || "").toLowerCase();
    if (!["translate", "rotate", "scale"].includes(nextMode)) return;
    this.transformControls.setMode(nextMode);
  }

  setTransformSpace(space) {
    const nextSpace = String(space || "").toLowerCase();
    if (!["local", "world"].includes(nextSpace)) return;
    this.transformSpace = nextSpace;
    this.transformControls.setSpace(nextSpace);
  }

  getTransformSpace() {
    return this.transformSpace;
  }

  setDragMode(enabled) {
    this.dragModeEnabled = !!enabled;
    this.dragControls.enabled = this.dragModeEnabled;
    this.controls.enabled = !this.dragModeEnabled && !this.isTransformDragging;
    this.updateSelectionVisuals();
    this.canvas.style.cursor = this.dragModeEnabled ? "grab" : "default";
  }

  hasObject(id) {
    if (id == null) return false;
    return this.objectMap.has(String(id));
  }

  hasObjects(ids) {
    if (!Array.isArray(ids) || !ids.length) return false;
    return ids.every((id) => this.hasObject(id));
  }

  getVisibleObjectIds() {
    return [...this.objectMap.entries()].filter(([, mesh]) => mesh.visible).map(([id]) => id);
  }

  getSelectionIds() {
    return [...this.selectedIds];
  }

  clearSelection(emit = true) {
    this.selectMultiple([], emit);
  }

  selectAll(emit = true) {
    this.selectMultiple([...this.objectMap.keys()], emit);
  }

  selectMultiple(ids, emit = true, primaryId = null) {
    const nextIds = Array.isArray(ids) ? ids.map((id) => String(id)) : [];
    const deduped = [];
    const seen = new Set();
    for (const id of nextIds) {
      if (!this.objectMap.has(id)) continue;
      if (seen.has(id)) continue;
      seen.add(id);
      deduped.push(id);
    }

    const targetPrimary =
      primaryId && seen.has(String(primaryId))
        ? String(primaryId)
        : deduped.includes(this.selectedId)
          ? this.selectedId
          : deduped[deduped.length - 1] || null;

    this.selectedIds = new Set(deduped);
    this.selectedId = targetPrimary;
    this.updateSelectionVisuals();

    if (emit) {
      const primaryEntity = this.selectedId ? this.objectMap.get(this.selectedId)?.userData?.entity : null;
      this.onSelect(primaryEntity || null);
      this.onSelectionChange(
        deduped.map((id) => this.objectMap.get(id)?.userData?.entity).filter(Boolean),
      );
    }
  }

  setObjectStateMap(stateMap) {
    this.objectStateMap = stateMap instanceof Map ? new Map(stateMap) : new Map();
    this.applyObjectStates();
  }

  updateScene(ir) {
    const entities = Array.isArray(ir?.entities) ? ir.entities : [];
    const renderableEntities = entities.filter((entity) => isRenderableEntity(entity));
    const incoming = new Set(renderableEntities.map((entity) => String(entity.id)));

    for (const [id, mesh] of this.objectMap.entries()) {
      if (!incoming.has(id)) {
        this.scene.remove(mesh);
        mesh.geometry?.dispose();
        mesh.userData.baseGeometry?.dispose();
        mesh.material?.dispose();
        this.objectMap.delete(id);
        this.selectedIds.delete(id);
        if (this.selectedId === id) this.selectedId = null;
      }
    }

    const precision = Number(ir?.metadata?.precision ?? this.currentPrecision);
    this.setPrecision(precision > 0 ? precision : this.currentPrecision);
    if (ir?.metadata?.unit_system && this.measurementUnit === "units") {
      const unitSystem = String(ir.metadata.unit_system).toLowerCase();
      this.measurementUnit = unitSystem === "imperial" ? "in" : "m";
    }

    for (const entity of renderableEntities) {
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
        const geometry = createGeometry(primitive, dimensions);
        mesh = new THREE.Mesh(
          geometry,
          new THREE.MeshStandardMaterial({ color: 0xc47a4e, roughness: 0.48, metalness: 0.06 }),
        );
        mesh.userData.entityId = id;
        mesh.userData.baseGeometry = geometry.clone();
        mesh.userData.booleanPreviewActive = false;
        this.scene.add(mesh);
        this.objectMap.set(id, mesh);
      } else {
        const prevSig = mesh.userData.geometrySignature;
        const nextSig = `${primitive}:${dimensions.join(":")}`;
        if (prevSig !== nextSig) {
          const geometry = createGeometry(primitive, dimensions);
          mesh.geometry.dispose();
          mesh.geometry = geometry;
          mesh.userData.baseGeometry?.dispose();
          mesh.userData.baseGeometry = geometry.clone();
          mesh.userData.booleanPreviewActive = false;
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
    this.applyObjectStates();
    this.applyDisplayMode();
    this.applySectionPlane();
    this.resetBooleanPreviewGeometry();
    this.applyBooleanConstraints(Array.isArray(ir?.constraints) ? ir.constraints : []);

    if (this.selectedId) {
      this.selectMultiple([...this.selectedIds], false, this.selectedId);
    } else if (this.selectedIds.size) {
      this.selectMultiple([...this.selectedIds], false);
    }
  }

  setDisplayMode(mode) {
    const nextMode = String(mode || "").toLowerCase();
    if (!["solid", "wireframe", "xray"].includes(nextMode)) return;
    this.displayMode = nextMode;
    this.applyDisplayMode();
  }

  setSectionState({ enabled, axis, offset } = {}) {
    if (typeof enabled === "boolean") this.sectionState.enabled = enabled;
    if (["x", "y", "z"].includes(String(axis || "").toLowerCase())) {
      this.sectionState.axis = String(axis).toLowerCase();
    }
    if (Number.isFinite(Number(offset))) {
      this.sectionState.offset = Number(offset);
    }
    this.applySectionPlane();
  }

  getSectionState() {
    return { ...this.sectionState };
  }

  focusSelection(ids) {
    const targets = Array.isArray(ids) ? ids.map((id) => this.objectMap.get(String(id))).filter(Boolean) : [];
    this.frameObjects(targets);
  }

  frameAll() {
    this.frameObjects([...this.objectMap.values()]);
  }

  frameSelected(ids = null) {
    const selectedIds = Array.isArray(ids) && ids.length ? ids : [...this.selectedIds];
    const targets = selectedIds.map((id) => this.objectMap.get(String(id))).filter(Boolean);
    this.frameObjects(targets);
  }

  setCameraPreset(preset, ids = null) {
    const key = String(preset || "").toLowerCase();
    const selectedIds = Array.isArray(ids) && ids.length ? ids : [...this.selectedIds];
    const targets =
      key === "all"
        ? [...this.objectMap.values()]
        : selectedIds.length
          ? selectedIds.map((id) => this.objectMap.get(String(id))).filter(Boolean)
          : [...this.objectMap.values()];
    const box = this.computeObjectsBounds(targets);
    if (!box) return;
    const center = box.getCenter(new THREE.Vector3());
    const size = box.getSize(new THREE.Vector3()).length() || 1;
    const distance = Math.max(size * 1.35, 4);

    const position = new THREE.Vector3();
    if (key === "front") position.set(center.x, center.y, center.z + distance);
    else if (key === "top") position.set(center.x, center.y + distance, center.z);
    else if (key === "right") position.set(center.x + distance, center.y, center.z);
    else position.set(center.x + distance * 0.9, center.y + distance * 0.8, center.z + distance * 0.9);

    this.controls.target.copy(center);
    this.camera.position.copy(position);
    this.controls.update();
  }

  async exportMeshes(format = "obj", ids = null) {
    const targetIds = Array.isArray(ids) && ids.length ? ids : [...this.objectMap.keys()];
    const meshes = targetIds.map((id) => this.objectMap.get(String(id))).filter(Boolean);
    if (!meshes.length) return null;
    const group = new THREE.Group();
    for (const mesh of meshes) {
      const clone = mesh.clone();
      clone.geometry = mesh.geometry.clone();
      group.add(clone);
    }

    const lower = String(format || "").toLowerCase();
    if (lower === "obj") {
      const module = await import("https://esm.sh/three@0.160.1/examples/jsm/exporters/OBJExporter.js");
      const exporter = new module.OBJExporter();
      return { format: "obj", data: exporter.parse(group), mime: "text/plain" };
    }
    if (lower === "stl") {
      const module = await import("https://esm.sh/three@0.160.1/examples/jsm/exporters/STLExporter.js");
      const exporter = new module.STLExporter();
      return { format: "stl", data: exporter.parse(group), mime: "model/stl" };
    }
    return null;
  }

  selectObject(id, emit = true) {
    if (!id) {
      this.selectMultiple([], emit);
      return;
    }
    this.selectMultiple([String(id)], emit, String(id));
  }

  focusObject(id) {
    const mesh = this.objectMap.get(String(id));
    if (!mesh) return;
    this.selectMultiple([String(id)], true, String(id));
    this.frameObjects([mesh]);
  }

  updateSelectionVisuals() {
    for (const [meshId, mesh] of this.objectMap.entries()) {
      const material = mesh.material;
      if (material && "emissive" in material) {
        const active = this.selectedIds.has(meshId);
        material.emissive.set(active ? 0x5e2a13 : 0x000000);
      }
    }

    const primary = this.selectedId ? this.objectMap.get(this.selectedId) : null;
    const primaryState = this.selectedId ? this.objectStateMap.get(this.selectedId) : null;
    const isLockedPrimary = !!primaryState?.locked;
    if (primary && !this.dragModeEnabled && !isLockedPrimary) {
      this.transformControls.attach(primary);
      this.transformControls.visible = true;
      this.transformControls.enabled = true;
    } else {
      this.transformControls.detach();
      this.transformControls.visible = false;
      this.transformControls.enabled = !isLockedPrimary;
    }
  }

  applyObjectStates() {
    let isolatedIds = null;
    for (const [id, state] of this.objectStateMap.entries()) {
      if (state?.isolate) {
        if (!isolatedIds) isolatedIds = new Set();
        isolatedIds.add(String(id));
      }
    }

    for (const [id, mesh] of this.objectMap.entries()) {
      const state = this.objectStateMap.get(id) || {};
      const hidden = !!state.hidden;
      const locked = !!state.locked;
      const isolatedOut = isolatedIds ? !isolatedIds.has(id) : false;
      const visible = !hidden && !isolatedOut;
      mesh.visible = visible;
      mesh.userData.locked = locked;

      const material = mesh.material;
      if (material && typeof material === "object") {
        material.depthWrite = !locked;
        if ("emissive" in material && locked) {
          material.emissive.setHex(0x1c3f6e);
        }
      }

      if (!visible && this.selectedIds.has(id)) {
        this.selectedIds.delete(id);
        if (this.selectedId === id) this.selectedId = null;
      }
    }
    this.updateSelectionVisuals();
  }

  applyDisplayMode() {
    for (const mesh of this.objectMap.values()) {
      const material = mesh.material;
      if (!material || typeof material !== "object") continue;
      material.wireframe = this.displayMode === "wireframe";
      if (this.displayMode === "xray") {
        material.transparent = true;
        material.opacity = 0.28;
        material.depthWrite = false;
      } else {
        material.transparent = false;
        material.opacity = 1;
        material.depthWrite = true;
      }
      material.needsUpdate = true;
    }
    this.updateSelectionVisuals();
  }

  applySectionPlane() {
    const axis = this.sectionState.axis || "y";
    const offset = Number(this.sectionState.offset || 0);
    const normal =
      axis === "x"
        ? new THREE.Vector3(-1, 0, 0)
        : axis === "z"
          ? new THREE.Vector3(0, 0, -1)
          : new THREE.Vector3(0, -1, 0);
    this.sectionPlane.set(normal, offset);

    const clipping = this.sectionState.enabled ? [this.sectionPlane] : [];
    for (const mesh of this.objectMap.values()) {
      const material = mesh.material;
      if (!material || typeof material !== "object") continue;
      material.clippingPlanes = clipping;
      material.needsUpdate = true;
    }
  }

  computeObjectsBounds(objects) {
    if (!Array.isArray(objects) || !objects.length) return null;
    const box = new THREE.Box3();
    let hasAny = false;
    for (const object of objects) {
      if (!object || !object.visible) continue;
      const itemBox = new THREE.Box3().setFromObject(object);
      if (itemBox.isEmpty()) continue;
      if (!hasAny) {
        box.copy(itemBox);
        hasAny = true;
      } else {
        box.union(itemBox);
      }
    }
    return hasAny ? box : null;
  }

  frameObjects(objects) {
    const box = this.computeObjectsBounds(objects);
    if (!box) return;
    const center = box.getCenter(new THREE.Vector3());
    const sphere = box.getBoundingSphere(new THREE.Sphere());
    const radius = Math.max(sphere.radius, 0.5);
    const distance = radius * 2.3;
    const direction = new THREE.Vector3().subVectors(this.camera.position, this.controls.target).normalize();
    if (!Number.isFinite(direction.lengthSq()) || direction.lengthSq() <= 1e-8) {
      direction.set(1, 0.8, 1).normalize();
    }
    this.controls.target.copy(center);
    this.camera.position.copy(center.clone().add(direction.multiplyScalar(distance)));
    this.controls.update();
  }

  emitTransform() {
    this.emitTransformForObject(this.transformControls.object);
  }

  onTransformObjectChange() {
    if (this.isApplyingSnap) return;
    const object = this.transformControls.object;
    if (!object) return;
    if (object.userData?.locked) return;
    this.applySnappingForObject(object);
    this.emitTransformForObject(object);
  }

  emitTransformForObject(object) {
    if (!object?.userData?.entity) return;
    if (object.userData?.locked) return;
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
    const locked = !!hit.object.userData?.locked;

    if (this.measureMode) {
      this.measurePoints.push(hit.point.clone());
      if (this.measurePoints.length === 2) {
        this.addDistanceMeasurement(this.measurePoints[0], this.measurePoints[1]);
      } else if (this.measurePoints.length === 3) {
        this.addAngleMeasurement(this.measurePoints[0], this.measurePoints[1], this.measurePoints[2]);
        this.measurePoints = [];
      }
      return;
    }

    if (!id || locked) return;

    const nextId = String(id);
    if (event.shiftKey) {
      const next = new Set(this.selectedIds);
      if (next.has(nextId)) next.delete(nextId);
      else next.add(nextId);
      this.selectMultiple([...next], true, nextId);
      return;
    }

    this.selectMultiple([nextId], true, nextId);
  }

  addDistanceMeasurement(start, end) {
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
    label.textContent = `${distance.toFixed(3)} ${this.measurementUnit}`;

    const tag = new CSS2DObject(label);
    tag.position.copy(center);
    this.scene.add(tag);

    this.annotations.push(line, tag);
  }

  addAngleMeasurement(a, b, c) {
    const rayOne = a.clone().sub(b);
    const rayTwo = c.clone().sub(b);
    const oneLength = rayOne.length();
    const twoLength = rayTwo.length();
    if (oneLength <= 1e-8 || twoLength <= 1e-8) return;

    const normalizedOne = rayOne.normalize();
    const normalizedTwo = rayTwo.normalize();
    const cosine = THREE.MathUtils.clamp(normalizedOne.dot(normalizedTwo), -1, 1);
    const angle = THREE.MathUtils.radToDeg(Math.acos(cosine));

    const guideOne = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([b, a]),
      new THREE.LineDashedMaterial({ color: 0x445f9f, dashSize: 0.2, gapSize: 0.12 }),
    );
    guideOne.computeLineDistances();
    const guideTwo = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([b, c]),
      new THREE.LineDashedMaterial({ color: 0x445f9f, dashSize: 0.2, gapSize: 0.12 }),
    );
    guideTwo.computeLineDistances();
    this.scene.add(guideOne);
    this.scene.add(guideTwo);

    const label = document.createElement("div");
    label.style.padding = "2px 6px";
    label.style.border = "1px solid rgba(68,95,159,0.35)";
    label.style.borderRadius = "999px";
    label.style.background = "rgba(255,255,255,0.9)";
    label.style.color = "#304679";
    label.style.fontFamily = "IBM Plex Mono, monospace";
    label.style.fontSize = "11px";
    label.textContent = `${angle.toFixed(2)}°`;

    const anchor = b
      .clone()
      .add(normalizedOne)
      .add(normalizedTwo)
      .multiplyScalar(0.5);
    const tag = new CSS2DObject(label);
    tag.position.copy(anchor);
    this.scene.add(tag);

    this.annotations.push(guideOne, guideTwo, tag);
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

  resetBooleanPreviewGeometry() {
    for (const mesh of this.objectMap.values()) {
      if (!mesh.userData.booleanPreviewActive) continue;
      const baseGeometry = mesh.userData.baseGeometry;
      if (!baseGeometry) {
        mesh.userData.booleanPreviewActive = false;
        continue;
      }
      mesh.geometry?.dispose();
      mesh.geometry = baseGeometry.clone();
      mesh.userData.booleanPreviewActive = false;
    }
  }

  applySnappingForObject(object) {
    if (!object) return;
    const transformMode = this.transformControls.getMode?.() || "translate";
    const supportsPositionSnap = this.dragModeEnabled || transformMode === "translate";
    if (!supportsPositionSnap) return;

    this.isApplyingSnap = true;
    try {
      if (this.snapEnabled) {
        object.position.set(
          snapScalar(object.position.x, this.currentPrecision),
          snapScalar(object.position.y, this.currentPrecision),
          snapScalar(object.position.z, this.currentPrecision),
        );
      }
      this.applyFeatureSnap(object);
    } finally {
      this.isApplyingSnap = false;
    }
  }

  applyFeatureSnap(object) {
    if (!this.vertexSnapEnabled && !this.edgeSnapEnabled) return;
    const snapRadius = Math.max(this.currentPrecision * 4, 0.2);
    let nearest = null;

    if (this.vertexSnapEnabled) {
      nearest = this.findNearestVertexPoint(object, snapRadius);
    }
    if (this.edgeSnapEnabled) {
      const edgeNearest = this.findNearestEdgePoint(object, snapRadius);
      if (!nearest || (edgeNearest && edgeNearest.distanceSq < nearest.distanceSq)) {
        nearest = edgeNearest;
      }
    }
    if (!nearest) return;
    object.position.copy(nearest.point);
  }

  findNearestVertexPoint(movingObject, radius) {
    const radiusSq = radius * radius;
    const origin = movingObject.position.clone();
    let nearest = null;
    const scratch = new THREE.Vector3();

    for (const mesh of this.objectMap.values()) {
      if (mesh === movingObject) continue;
      const positions = mesh.geometry?.attributes?.position;
      if (!positions) continue;
      mesh.updateWorldMatrix(true, false);
      const step = Math.max(1, Math.floor(positions.count / 300));
      for (let i = 0; i < positions.count; i += step) {
        scratch.fromBufferAttribute(positions, i).applyMatrix4(mesh.matrixWorld);
        const distanceSq = scratch.distanceToSquared(origin);
        if (distanceSq > radiusSq) continue;
        if (!nearest || distanceSq < nearest.distanceSq) {
          nearest = { point: scratch.clone(), distanceSq };
        }
      }
    }
    return nearest;
  }

  findNearestEdgePoint(movingObject, radius) {
    const radiusSq = radius * radius;
    const origin = movingObject.position.clone();
    let nearest = null;
    const box = new THREE.Box3();
    const closest = new THREE.Vector3();

    for (const mesh of this.objectMap.values()) {
      if (mesh === movingObject) continue;
      box.setFromObject(mesh);
      if (box.isEmpty()) continue;
      const corners = boxCorners(box);
      for (const [start, end] of boxEdges(corners)) {
        closest.copy(closestPointOnSegment(origin, start, end));
        const distanceSq = closest.distanceToSquared(origin);
        if (distanceSq > radiusSq) continue;
        if (!nearest || distanceSq < nearest.distanceSq) {
          nearest = { point: closest.clone(), distanceSq };
        }
      }
    }
    return nearest;
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
          targetMesh.userData.booleanPreviewActive = true;
        }
      } catch {
        // Keep base geometry if CSG preview fails.
      }
    }
  }

  updateGrid(precision) {
    const spacing = Math.max(Number(precision) || 1, 1e-6);
    const divisions = 200;
    const size = spacing * divisions;
    this.scene.remove(this.grid);
    this.grid.geometry.dispose();
    this.grid.material.dispose();
    this.grid = new THREE.GridHelper(size, divisions, 0x9f9581, 0xc6bca8);
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

function isRenderableEntity(entity) {
  const geometry = getComponent(entity, "geometry");
  const solid = getComponent(entity, "solid");
  return hasRenderablePrimitive(geometry) || hasRenderablePrimitive(solid);
}

function hasRenderablePrimitive(component) {
  const primitive = getProperty(component, "primitive", null);
  if (primitive == null) return false;
  const normalized = String(primitive).trim().toLowerCase();
  return normalized.length > 0;
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

function snapScalar(value, precision) {
  if (!Number.isFinite(value) || !Number.isFinite(precision) || precision <= 0) return value;
  return Math.round(value / precision) * precision;
}

function boxCorners(box) {
  const { min, max } = box;
  return [
    new THREE.Vector3(min.x, min.y, min.z),
    new THREE.Vector3(max.x, min.y, min.z),
    new THREE.Vector3(min.x, max.y, min.z),
    new THREE.Vector3(max.x, max.y, min.z),
    new THREE.Vector3(min.x, min.y, max.z),
    new THREE.Vector3(max.x, min.y, max.z),
    new THREE.Vector3(min.x, max.y, max.z),
    new THREE.Vector3(max.x, max.y, max.z),
  ];
}

function boxEdges(corners) {
  return [
    [corners[0], corners[1]],
    [corners[2], corners[3]],
    [corners[4], corners[5]],
    [corners[6], corners[7]],
    [corners[0], corners[2]],
    [corners[1], corners[3]],
    [corners[4], corners[6]],
    [corners[5], corners[7]],
    [corners[0], corners[4]],
    [corners[1], corners[5]],
    [corners[2], corners[6]],
    [corners[3], corners[7]],
  ];
}

function closestPointOnSegment(point, start, end) {
  const segment = new THREE.Vector3().subVectors(end, start);
  const pointOffset = new THREE.Vector3().subVectors(point, start);
  const lengthSq = segment.lengthSq();
  if (lengthSq <= 1e-12) return start.clone();
  const t = THREE.MathUtils.clamp(pointOffset.dot(segment) / lengthSq, 0, 1);
  return start.clone().add(segment.multiplyScalar(t));
}
