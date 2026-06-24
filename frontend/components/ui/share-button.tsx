"use client";

import React from "react";
import { Share2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import SocialIcon, { SocialIconId } from "@/components/ui/SocialIcon";

export interface ShareButtonProps {
  /** The URL to share */
  url: string;
  /** The text/title to share */
  title?: string;
  /** Additional description text (for some social networks) */
  text?: string;
  /** Custom class names */
  className?: string;
  /** Which social networks to include (defaults to all) */
  networks?: SocialIconId[];
}

const defaultNetworks: SocialIconId[] = ["x", "telegram", "reddit", "discord"];

function getShareUrl(network: SocialIconId, url: string, title?: string, text?: string) {
  const encodedUrl = encodeURIComponent(url);
  const encodedTitle = title ? encodeURIComponent(title) : "";
  const encodedText = text ? encodeURIComponent(text) : "";

  switch (network) {
    case "x":
      return `https://twitter.com/intent/tweet?url=${encodedUrl}${encodedTitle ? `&text=${encodedTitle}` : ""}`;
    case "telegram":
      return `https://t.me/share/url?url=${encodedUrl}${encodedTitle ? `&text=${encodedTitle}` : ""}`;
    case "reddit":
      return `https://reddit.com/submit?url=${encodedUrl}${encodedTitle ? `&title=${encodedTitle}` : ""}`;
    case "discord":
      // Discord doesn't have a direct share link, so just copy the URL
      return null;
    default:
      return null;
  }
}

export function ShareButton({
  url,
  title,
  text,
  className,
  networks = defaultNetworks,
}: ShareButtonProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const buttonRef = React.useRef<HTMLButtonElement>(null);
  const dropdownRef = React.useRef<HTMLDivElement>(null);

  const handleShare = async () => {
    if (navigator.share) {
      try {
        await navigator.share({
          title,
          text,
          url,
        });
      } catch (err) {
        // User canceled or share failed, open custom dropdown
        setIsOpen(!isOpen);
      }
    } else {
      setIsOpen(!isOpen);
    }
  };

  const handleNetworkClick = (network: SocialIconId) => {
    const shareUrl = getShareUrl(network, url, title, text);
    if (shareUrl) {
      window.open(shareUrl, "_blank", "noopener,noreferrer");
    } else if (network === "discord") {
      // For Discord, copy to clipboard
      navigator.clipboard.writeText(url);
      // TODO: Maybe show a toast?
    }
    setIsOpen(false);
  };

  // Close dropdown when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  return (
    <div className="relative">
      <Button
        ref={buttonRef}
        onClick={handleShare}
        variant="tertiary"
        size="icon"
        className={cn(className)}
      >
        <Share2 className="w-4 h-4" />
        <span className="sr-only">Share</span>
      </Button>

      {isOpen && (
        <div
          ref={dropdownRef}
          className="absolute right-0 mt-2 z-50 rounded-lg border border-input bg-background p-2 shadow-lg"
        >
          <div className="flex items-center gap-2">
            {networks.map((network) => (
              <button
                key={network}
                type="button"
                onClick={() => handleNetworkClick(network)}
                className="p-2 rounded-md hover:bg-accent transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                aria-label={`Share on ${network}`}
              >
                <SocialIcon id={network} label={network} className="w-5 h-5" />
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
