"use client";

import { useState } from "react";
import { Button, Input } from "@/components/ui";

interface ProfileFormValues {
  username: string;
  email: string;
  bio: string;
}

interface ProfileFormErrors {
  username?: string;
  email?: string;
  bio?: string;
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function validate(values: ProfileFormValues): ProfileFormErrors {
  const errors: ProfileFormErrors = {};
  if (values.username.length < 3 || values.username.length > 32) {
    errors.username = "Username must be 3–32 characters.";
  }
  if (values.email && !EMAIL_RE.test(values.email)) {
    errors.email = "Enter a valid email address.";
  }
  if (values.bio.length > 160) {
    errors.bio = "Bio must be 160 characters or fewer.";
  }
  return errors;
}

export function ProfileForm() {
  const [values, setValues] = useState<ProfileFormValues>({
    username: "",
    email: "",
    bio: "",
  });
  const [errors, setErrors] = useState<ProfileFormErrors>({});
  const [saved, setSaved] = useState(false);

  function handleChange(field: keyof ProfileFormValues, value: string) {
    setValues((prev) => ({ ...prev, [field]: value }));
    setErrors((prev) => ({ ...prev, [field]: undefined }));
    setSaved(false);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const errs = validate(values);
    if (Object.keys(errs).length > 0) {
      setErrors(errs);
      return;
    }
    // TODO: wire up to API
    setSaved(true);
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="space-y-6">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold text-white">Profile</h2>
        <p className="text-xs text-zinc-500">Update your public profile information.</p>
      </div>

      <div className="space-y-4">
        <Input
          label="Username"
          placeholder="satoshi"
          value={values.username}
          onChange={(e) => handleChange("username", e.target.value)}
          error={errors.username}
          helperText="3–32 characters."
          autoComplete="username"
        />
        <Input
          label="Email"
          type="email"
          placeholder="you@example.com"
          value={values.email}
          onChange={(e) => handleChange("email", e.target.value)}
          error={errors.email}
          autoComplete="email"
        />
        <div className="space-y-2">
          <label className="text-sm font-medium text-zinc-200">
            Bio
          </label>
          <textarea
            rows={3}
            placeholder="Tell the community about yourself…"
            value={values.bio}
            onChange={(e) => handleChange("bio", e.target.value)}
            className="w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm text-white placeholder:text-zinc-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 resize-none"
          />
          {errors.bio && (
            <p className="text-sm text-destructive" role="alert">{errors.bio}</p>
          )}
          <p className="text-xs text-zinc-500 text-right">{values.bio.length}/160</p>
        </div>
      </div>

      <div className="flex items-center gap-3">
        <Button type="submit" size="medium">Save changes</Button>
        {saved && (
          <span className="text-sm text-green-400" role="status">Saved!</span>
        )}
      </div>
    </form>
  );
}
