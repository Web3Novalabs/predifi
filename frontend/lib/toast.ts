import { toast } from "sonner";

type ToastType = "success" | "error" | "info" | "warning";

export const showToast = (
  message: string,
  type: ToastType = "info",
  options?: {
    duration?: number;
    position?:
      | "top-left"
      | "top-center"
      | "top-right"
      | "bottom-left"
      | "bottom-center"
      | "bottom-right";
  }
) => {
  const { duration = 5000, position = "top-center" } = options || {};

  switch (type) {
    case "success":
      toast.success(message, { duration, position });
      break;
    case "error":
      toast.error(message, { duration, position });
      break;
    case "warning":
      toast.warning(message, { duration, position });
      break;
    case "info":
      toast.info(message, { duration, position }); // Simplified
      break;
    default:
      toast(message, { duration, position });
      break;
  }
};

export const showAsyncToast = async <T>(
  promise: Promise<T>,
  loadingMessage: string,
  successMessage: string,
  errorMessage: string
) => {
  toast.promise(promise, {
    loading: loadingMessage,
    success: successMessage,
    error: errorMessage,
  });
};
