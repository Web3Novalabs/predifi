/**
 * SocialIcon
 *
 * Renders a social-media icon from the SVG sprite at /sprite.svg.
 * Using an SVG sprite means the browser downloads a single file and
 * references individual symbols via <use>, avoiding one HTTP request
 * per icon and keeping the DOM lean.
 *
 * Usage:
 *   <SocialIcon id="telegram" label="Telegram" className="w-6 h-6" />
 *
 * Available ids: "telegram" | "reddit" | "x" | "discord"
 */

import React from "react";

export type SocialIconId = "telegram" | "reddit" | "x" | "discord";

interface SocialIconProps {
  /** The icon identifier — must match a <symbol id="icon-{id}"> in /sprite.svg */
  id: SocialIconId;
  /** Accessible label for screen readers */
  label: string;
  className?: string;
}

export default function SocialIcon({ id, label, className = "w-6 h-6" }: SocialIconProps) {
  return (
    <svg
      className={className}
      aria-label={label}
      role="img"
      focusable="false"
    >
      <use href={`/sprite.svg#icon-${id}`} />
    </svg>
  );
}
