"use client";

import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { Sun } from "lucide-react";
import { IoMdMoon } from "react-icons/io";

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
      className="p-1 rounded-full  transition"
    >
      {resolvedTheme === "dark" ? (
        <IoMdMoon  className="text-black h-[50px] w-[25px] " />
       
      ) : (
        <Sun className="text-yellow-600" />
        
      )}
    </button>
  );
}
