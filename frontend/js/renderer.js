export class ThreeRenderer {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.scene = null;
        this.camera = null;
        this.renderer = null;
        this.controls = null;
        this.objects = [];
        this.animations = [];
        this.motions = [];
        this.timelineEvents = [];
        this.isPlaying = true;
        this.currentTime = 0;
        this.lastTimestamp = null;
        
        this.initThreeJS();
        this.setupAnimation();
    }
    
    initThreeJS() {
        // Scene setup
        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x1a1a2e);
        this.scene.fog = new THREE.Fog(0x1a1a2e, 100, 500);
        
        // Camera setup
        const width = this.canvas.clientWidth;
        const height = this.canvas.clientHeight;
        this.camera = new THREE.PerspectiveCamera(75, width / height, 0.1, 1000);
        this.camera.position.set(0, 10, 15);
        this.camera.lookAt(0, 0, 0);
        
        // Renderer setup
        this.renderer = new THREE.WebGLRenderer({ 
            canvas: this.canvas, 
            antialias: true,
            alpha: true 
        });
        this.renderer.setPixelRatio(window.devicePixelRatio);
        this.renderer.shadowMap.enabled = true;
        // ensure canvas has correct initial size (some browsers report 0 initially)
        this.renderer.setSize(width || 800, height || 600);
        // schedule a second resize after layout settles
        setTimeout(() => this.onWindowResize(), 50);
        console.log('[ThreeRenderer] init: canvas size', { width: this.canvas.clientWidth, height: this.canvas.clientHeight });
        
        // Lighting
        const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
        this.scene.add(ambientLight);
        
        const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
        directionalLight.position.set(10, 20, 10);
        directionalLight.castShadow = true;
        directionalLight.shadow.mapSize.width = 2048;
        directionalLight.shadow.mapSize.height = 2048;
        this.scene.add(directionalLight);
        
        // Grid helper
        const gridHelper = new THREE.GridHelper(50, 50, 0x444444, 0x222222);
        this.scene.add(gridHelper);
        
        // Axes helper
        const axesHelper = new THREE.AxesHelper(15);
        this.scene.add(axesHelper);
        
        // Handle window resize
        window.addEventListener('resize', () => this.onWindowResize());
        // Initialize simple pointer-based orbit controls
        this.initControls();
    }

    initControls() {
        this.cameraTarget = new THREE.Vector3(0, 0, 0);
        // spherical coords: radius, theta (azimuth), phi (inclination)
        const pos = this.camera.position.clone().sub(this.cameraTarget);
        const radius = pos.length();
        const theta = Math.atan2(pos.x, pos.z);
        const phi = Math.acos(Math.max(-1, Math.min(1, pos.y / radius)));
        this.cameraSpherical = { radius, theta, phi };

        this._isDragging = false;
        this._dragStart = { x: 0, y: 0 };
        this._sphericalStart = { ...this.cameraSpherical };

        const canvas = this.canvas;
        canvas.style.touchAction = 'none';

        canvas.addEventListener('pointerdown', (e) => {
            this._isDragging = true;
            this._dragStart.x = e.clientX;
            this._dragStart.y = e.clientY;
            this._sphericalStart = { ...this.cameraSpherical };
            canvas.setPointerCapture(e.pointerId);
        });

        canvas.addEventListener('pointermove', (e) => {
            if (!this._isDragging) return;
            const dx = (e.clientX - this._dragStart.x) / Math.max(1, canvas.clientWidth);
            const dy = (e.clientY - this._dragStart.y) / Math.max(1, canvas.clientHeight);
            const ROTATE_SPEED = Math.PI * 1.2; // radians per full width
            this.cameraSpherical.theta = this._sphericalStart.theta - dx * ROTATE_SPEED;
            this.cameraSpherical.phi = this._sphericalStart.phi - dy * ROTATE_SPEED;
            // clamp phi
            const EPS = 0.01;
            this.cameraSpherical.phi = Math.max(EPS, Math.min(Math.PI - EPS, this.cameraSpherical.phi));
            this.updateCameraFromSpherical();
        });

        const endDrag = (e) => {
            this._isDragging = false;
            try { canvas.releasePointerCapture(e.pointerId); } catch (err) {}
        };
        canvas.addEventListener('pointerup', endDrag);
        canvas.addEventListener('pointercancel', endDrag);

        // wheel zoom
        canvas.addEventListener('wheel', (e) => {
            e.preventDefault();
            const delta = e.deltaY * 0.01;
            this.cameraSpherical.radius *= 1 + delta;
            this.cameraSpherical.radius = Math.max(0.5, Math.min(200, this.cameraSpherical.radius));
            this.updateCameraFromSpherical();
        }, { passive: false });
    }

    updateCameraFromSpherical() {
        const s = this.cameraSpherical;
        const sinPhi = Math.sin(s.phi);
        const x = s.radius * sinPhi * Math.sin(s.theta);
        const y = s.radius * Math.cos(s.phi);
        const z = s.radius * sinPhi * Math.cos(s.theta);
        this.camera.position.set(this.cameraTarget.x + x, this.cameraTarget.y + y, this.cameraTarget.z + z);
        this.camera.lookAt(this.cameraTarget);
    }
    
    onWindowResize() {
        const width = this.canvas.clientWidth;
        const height = this.canvas.clientHeight;
        
        this.camera.aspect = width / height;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(width, height);
    }
    
    setupAnimation() {
        const animate = (timestamp) => {
            requestAnimationFrame(animate);

            if (this.lastTimestamp === null) this.lastTimestamp = timestamp;
            const deltaMs = timestamp - this.lastTimestamp;
            const delta = Math.min(deltaMs / 1000, 0.1);
            this.lastTimestamp = timestamp;

            if (this.isPlaying) {
                this.currentTime += delta;
                this.updateAnimations(delta);
            }

            this.renderer.render(this.scene, this.camera);
            // update UI time and fps if present
            try {
                const fpsEl = document.getElementById('fpsCounter');
                const timeEl = document.getElementById('timeDisplay');
                if (fpsEl && delta > 0) fpsEl.textContent = Math.round(1 / delta);
                if (timeEl) timeEl.textContent = this.currentTime.toFixed(2) + 's';
            } catch (e) {
                // ignore DOM errors in non-browser contexts
            }
        };
        requestAnimationFrame(animate);
    }

    setPlaying(flag) {
        this.isPlaying = !!flag;
    }

    updateAnimations(delta) {
        if (!this.timelineEvents || this.timelineEvents.length === 0) return;

        this.timelineEvents.forEach(evt => {
            const { start, duration, motion } = evt;
            const t = this.currentTime;
            if (t < start || t > (start + duration)) return;

            if (!motion) return;
            if (motion.motion_type === 'rotation') {
                const targetName = motion.target_entity;
                const axis = (motion.parameters?.axis?.Vector3) || motion.parameters?.axis || [0,1,0];
                const speed = motion.parameters?.speed?.Number || motion.parameters?.speed || 0;

                const obj = this.findObjectByName(targetName);
                if (!obj) return;

                const ax = new THREE.Vector3(axis[0], axis[1], axis[2]).normalize();
                const angle = speed * delta;
                const q = new THREE.Quaternion();
                q.setFromAxisAngle(ax, angle);
                obj.quaternion.premultiply(q);
            }
        });
    }

    findObjectByName(name) {
        return this.objects.find(o => o.name === name || o.userData?.id === name || o.userData?.id === String(name));
    }
    
    async loadScene(irScene) {
        // Clear previous objects
        this.clearScene();
        
        if (!irScene) {
            console.error('No scene data provided');
            return;
        }
        
        try {
            // Parse entities from IR scene
            if (irScene.entities) {
                // Handle both list and dict formats
                const entitiesList = Array.isArray(irScene.entities) 
                    ? irScene.entities 
                    : Object.values(irScene.entities);
                
                entitiesList.forEach((entity) => {
                    const name = entity.id || 'unknown';
                    this.createEntityMesh(name, entity);
                });
            }
            
            // Load motions/animations (motions + timelines)
            if (irScene.motions || irScene.timelines) {
                this.loadMotions(irScene.motions || [], irScene.timelines || []);
            }
            
            // Update object count
            document.getElementById('objectCount').textContent = this.objects.length;
            console.log('[ThreeRenderer] loadScene: created objects=', this.objects.length, 'entities=', irScene.entities);
            
            // Fit camera to scene
            this.fitCameraToScene();
            // reset timeline playback state for new scene
            this.currentTime = 0;
            this.lastTimestamp = null;
            
        } catch (error) {
            console.error('Error loading scene:', error);
            throw error;
        }
    }
    
    createEntityMesh(name, entity) {
        let mesh = null;
        
        try {
            const components = entity.components || {};
            const transform = components.transform || {};
            const geometry = components.geometry || {};
            
            // Extract position, rotation, scale from nested properties structure
            const transformProps = transform.properties || {};
            const position = this.extractVector3(transformProps.position) || [0, 0, 0];
            const rotation = this.extractVector3(transformProps.rotation) || [0, 0, 0];
            const scale = this.extractVector3(transformProps.scale) || [1, 1, 1];
            
            // Get geometry primitive
            const geometryProps = geometry.properties || {};
            const primitiveObj = geometryProps.primitive;
            const primitive = primitiveObj?.Identifier || primitiveObj || 'cube';
            
            // Create geometry based on primitive type
            let geom = null;
            
            switch (primitive) {
                case 'cube':
                    geom = new THREE.BoxGeometry(scale[0], scale[1], scale[2]);
                    break;
                case 'sphere':
                    geom = new THREE.SphereGeometry(scale[0], 32, 32);
                    break;
                case 'cylinder':
                    geom = new THREE.CylinderGeometry(scale[0], scale[0], scale[1], 32);
                    break;
                case 'cone':
                    geom = new THREE.ConeGeometry(scale[0], scale[1], 32);
                    break;
                case 'plane':
                    geom = new THREE.PlaneGeometry(scale[0], scale[1]);
                    break;
                default:
                    geom = new THREE.BoxGeometry(1, 1, 1);
            }
            
            // Create material
            const material = new THREE.MeshPhongMaterial({
                color: this.getRandomColor(),
                shininess: 100,
                wireframe: false
            });
            
            // Create mesh
            mesh = new THREE.Mesh(geom, material);
            mesh.castShadow = true;
            mesh.receiveShadow = true;
            
            // Apply transform
            mesh.position.set(position[0], position[1], position[2]);
            mesh.rotation.set(rotation[0], rotation[1], rotation[2]);
            mesh.name = name;
            
            // Store entity data for animation
            mesh.userData = entity;
            
            // Add to scene
            this.scene.add(mesh);
            this.objects.push(mesh);
            
            console.log(`Created entity: ${name} (${primitive})`, { position, rotation, scale });
            
        } catch (error) {
            console.error(`Error creating entity ${name}:`, error);
        }
    }
    
    extractVector3(vectorData) {
        if (!vectorData) return null;
        if (Array.isArray(vectorData)) return vectorData;
        if (vectorData.Vector3) return vectorData.Vector3;
        if (vectorData.Array) return vectorData.Array;
        return null;
    }
    
    getRandomColor() {
        const colors = [
            0xff6b6b, 0x4ecdc4, 0x45b7d1, 0xf7b731,
            0x845ef7, 0x6bff6b, 0xffd54f, 0xff6b8a
        ];
        return colors[Math.floor(Math.random() * colors.length)];
    }
    
    loadMotions(motions, timelines) {
        this.motions = motions || [];
        this.timelineEvents = [];

        const motionMap = {};
        (this.motions || []).forEach(m => { motionMap[m.id] = m; });

        (timelines || []).forEach(tl => {
            (tl.events || []).forEach(ev => {
                const motion = motionMap[ev.motion_id] || null;
                if (!motion) return;
                const start = ev.start_time || ev.start || 0;
                const duration = ev.duration || 0;
                this.timelineEvents.push({ start, duration, motion });
            });
        });

        console.log(`Loaded ${this.motions.length} motions and ${this.timelineEvents.length} timeline events`);
    }
    
    clearScene() {
        // Remove objects from scene
        this.objects.forEach(obj => {
            this.scene.remove(obj);
            obj.geometry.dispose();
            obj.material.dispose();
        });
        this.objects = [];
        this.motions = [];
        this.timelineEvents = [];
        
        // Reset camera
        this.camera.position.set(0, 10, 15);
        this.camera.lookAt(0, 0, 0);
    }
    
    fitCameraToScene() {
        const box = new THREE.Box3().setFromObject(this.scene);
        const size = box.getSize(new THREE.Vector3()).length();
        const center = box.getCenter(new THREE.Vector3());
        
        const fov = this.camera.fov * (Math.PI / 180);
        
        let cameraZ;
        if (!Number.isFinite(size) || size === 0) {
            cameraZ = 20;
        } else {
            cameraZ = Math.abs(size / (2 * Math.tan(fov / 2)));
        }
        
        this.camera.position.z = cameraZ + (size * 0.5);
        this.camera.lookAt(center || new THREE.Vector3(0, 0, 0));
    }

    
    resetCamera() {
        this.camera.position.set(0, 10, 15);
        this.camera.lookAt(0, 0, 0);
    }
    
    reset() {
        this.clearScene();
    }
}