"use client";

import Image from "next/image";
import { forwardRef, type InputHTMLAttributes } from "react";
import type { FieldError } from "react-hook-form";

interface ImageUploadProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "className"> {
  imageUrl: string | null;
  error?: FieldError;
}

const ImageUpload = forwardRef<HTMLInputElement, ImageUploadProps>(
  ({ imageUrl, error, ...props }, ref) => {
    return (
      <div className="flex flex-col items-center gap-2">
        <div className="relative">
          {imageUrl ? (
            <Image
              className="w-[111px] h-[111px] rounded-full object-cover"
              src={imageUrl || "/placeholder.svg"}
              alt="pool demo"
              width={111}
              height={111}
            />
          ) : (
            <div className="w-[111px] h-[111px] rounded-full bg-[#373737] flex items-center justify-center text-gray-400">
              <span className="text-xs">No image</span>
            </div>
          )}
        </div>
        <button
          className="relative text-sm text-gray-300 hover:text-white transition-colors cursor-pointer"
          type="button"
        >
          <span className="cursor-pointer">Add a picture</span>
          <input
            ref={ref}
            type="file"
            className="border opacity-0 w-full h-full absolute top-0 left-0 cursor-pointer"
            accept="image/*"
            {...props}
          />
        </button>
        {error && <p className="text-red-500 text-xs mt-1">{error.message}</p>}
      </div>
    );
  }
);

ImageUpload.displayName = "ImageUpload";

export default ImageUpload;
