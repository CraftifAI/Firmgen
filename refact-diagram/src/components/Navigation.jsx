import { Link, useLocation } from "react-router-dom";
import { ArrowLeft, Home } from "lucide-react";
import { motion } from "framer-motion";

export default function Navigation() {
  const location = useLocation();
  const isHome = location.pathname === "/";

  if (isHome) return null;

  return (
    <motion.nav
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      className="bg-neutral-900/95 backdrop-blur-sm border-b border-cyan-500/30 sticky top-0 z-50"
    >
      <div className="max-w-6xl mx-auto px-6 py-4 flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <Link
            to="/"
            className="flex items-center space-x-2 text-cyan-300 hover:text-cyan-400 transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
            <span>Back to Diagram</span>
          </Link>
          <div className="h-6 w-px bg-neutral-600"></div>
          <Link
            to="/"
            className="flex items-center space-x-2 text-neutral-300 hover:text-neutral-200 transition-colors"
          >
            <Home className="w-4 h-4" />
            <span>Home</span>
          </Link>
        </div>
        
        <div className="text-sm text-neutral-400">
          {location.pathname.split('/').pop().replace(/-/g, ' ').replace(/\b\w/g, l => l.toUpperCase())}
        </div>
      </div>
    </motion.nav>
  );
}
