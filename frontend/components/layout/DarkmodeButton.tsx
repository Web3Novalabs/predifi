"use client";

import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { Sun, Moon } from "lucide-react";

export default function DarkModeToggle() {
  const { resolvedTheme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) return null; 

  return (
    <button
      onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
      className="p-1 rounded-full bg-gray-200 dark:bg-gray-800 transition"
    >
      {resolvedTheme === "dark" ? (
        <Moon className="text-gray-400" />
       
      ) : (
        <Sun className="text-yellow-600" />
        
      )}
    </button>
  );
}
