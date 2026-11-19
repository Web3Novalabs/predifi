"use client";

import { useState } from "react";
import { Button, CopyButton, Input, useToastActions } from "@/components/ui";
import { ShieldCheck } from "lucide-react";
import { useWallet } from "@/lib/context/WalletContext";

/** Shorten a Stellar address for display: `GABCDE…WXYZ`. */
export function truncateAddress(address: string): string {
  return address.length <= 12
    ? address
    : `${address.slice(0, 6)}…${address.slice(-4)}`;
}

interface PasswordValues {
  current: string;
  next: string;
  confirm: string;
}

interface PasswordErrors {
  current?: string;
  next?: string;
  confirm?: string;
}

function validate(values: PasswordValues): PasswordErrors {
  const errors: PasswordErrors = {};
  if (!values.current) errors.current = "Current password is required.";
  if (values.next.length < 8) errors.next = "Password must be at least 8 characters.";
  if (values.next !== values.confirm) errors.confirm = "Passwords do not match.";
  return errors;
}

export function SecuritySettings() {
  const { address, connect, disconnect, isConnecting } = useWallet();
  const { addToast } = useToastActions();
  const [values, setValues] = useState<PasswordValues>({
    current: "",
    next: "",
    confirm: "",
  });
  const [errors, setErrors] = useState<PasswordErrors>({});
  const [saved, setSaved] = useState(false);

  function handleDisconnect() {
    disconnect();
    addToast({
      variant: "success",
      title: "Wallet disconnected",
      description: "Your wallet has been disconnected.",
    });
  }

  function handleChange(field: keyof PasswordValues, value: string) {
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
    <div className="space-y-6">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold text-white">Security</h2>
        <p className="text-xs text-zinc-500">Manage your wallet connection and authentication settings.</p>
      </div>

      {/* Wallet info */}
      <div className="flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-800/40 px-4 py-3">
        <ShieldCheck className="h-5 w-5 text-primary shrink-0" aria-hidden="true" />
        {address ? (
          <>
            <div className="min-w-0">
              <p className="text-sm font-medium text-white">Connected wallet</p>
              <div className="flex items-center gap-2 text-xs text-zinc-500">
                <span title={address} data-testid="wallet-address">
                  {truncateAddress(address)}
                </span>
                <CopyButton
                  text={address}
                  size="xs"
                  aria-label="Copy wallet address"
                  copyOptions={{ successDescription: "Wallet address copied to clipboard" }}
                />
              </div>
            </div>
            <Button
              type="button"
              variant="tertiary"
              size="small"
              className="ml-auto shrink-0"
              onClick={handleDisconnect}
            >
              Disconnect
            </Button>
          </>
        ) : (
          <>
            <div className="min-w-0">
              <p className="text-sm font-medium text-white">No wallet connected</p>
              <p className="text-xs text-zinc-500">
                Connect a wallet to manage your connection.
              </p>
            </div>
            <Button
              type="button"
              size="small"
              className="ml-auto shrink-0"
              disabled={isConnecting}
              onClick={() => void connect()}
            >
              {isConnecting ? "Connecting…" : "Connect wallet"}
            </Button>
          </>
        )}
      </div>

      {/* Change password */}
      <form onSubmit={handleSubmit} noValidate className="space-y-4">
        <p className="text-sm font-medium text-zinc-200">Change password</p>
        <Input
          label="Current password"
          type="password"
          value={values.current}
          onChange={(e) => handleChange("current", e.target.value)}
          error={errors.current}
          autoComplete="current-password"
        />
        <Input
          label="New password"
          type="password"
          value={values.next}
          onChange={(e) => handleChange("next", e.target.value)}
          error={errors.next}
          helperText="At least 8 characters."
          autoComplete="new-password"
        />
        <Input
          label="Confirm new password"
          type="password"
          value={values.confirm}
          onChange={(e) => handleChange("confirm", e.target.value)}
          error={errors.confirm}
          autoComplete="new-password"
        />
        <div className="flex items-center gap-3">
          <Button type="submit" size="medium">Update password</Button>
          {saved && (
            <span className="text-sm text-green-400" role="status">Updated!</span>
          )}
        </div>
      </form>
    </div>
  );
}
