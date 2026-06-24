import { Suspense, useEffect, useRef } from "react";
import { Canvas, useFrame } from "@react-three/fiber";
import { useGLTF, useAnimations, Environment, OrbitControls } from "@react-three/drei";

function Model() {
  const group = useRef();
  const { scene, animations } = useGLTF(
    "/free__lamborghini_terzo_millennio_wind_tunnel.glb"
  );
  const { actions, names } = useAnimations(animations, group);

  // Play every animation clip the GLB ships with.
  useEffect(() => {
    names.forEach((name) => {
      const action = actions[name];
      if (action) {
        action.reset().play();
      }
    });
  }, [actions, names]);

  return <primitive ref={group} object={scene} />;
}

export default function WindTunnelModel() {
  return (
    <Canvas
      camera={{ position: [0, 1.5, 6], fov: 45 }}
      gl={{ antialias: true }}
      style={{ position: "absolute", inset: 0, width: "100vw", height: "100vh" }}
    >
      <ambientLight intensity={0.4} />
      <directionalLight position={[5, 8, 5]} intensity={1.2} castShadow />
      <directionalLight position={[-5, 4, -5]} intensity={0.4} />
      <Environment preset="studio" background={false} />

      <Suspense fallback={null}>
        <Model />
      </Suspense>

      <OrbitControls
        enableZoom={true}
        enablePan={false}
        autoRotate={false}
        minDistance={2}
        maxDistance={20}
        minPolarAngle={Math.PI / 6}
        maxPolarAngle={Math.PI / 2}
      />
    </Canvas>
  );
}
