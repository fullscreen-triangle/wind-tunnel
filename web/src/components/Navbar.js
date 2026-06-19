import Link from "next/link";
import { useRouter } from "next/router";

const Navbar = () => {
  const router = useRouter();

  return (
    <header className="fixed top-0 left-0 w-full z-50 flex items-center justify-between px-10 py-6">
      <span className="text-white text-sm font-mono tracking-widest uppercase opacity-70">
        Wind Tunnel
      </span>
      <nav>
        <Link
          href="/sandbox"
          className={`text-sm font-mono tracking-widest uppercase transition-opacity duration-200
            ${router.asPath === "/sandbox" ? "opacity-100 text-white" : "opacity-50 text-white hover:opacity-100"}
          `}
        >
          Sandbox
        </Link>
      </nav>
    </header>
  );
};

export default Navbar;
