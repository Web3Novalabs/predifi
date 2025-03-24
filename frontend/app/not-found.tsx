import Image from "next/image";
import Link from "next/link";

export default function NotFound() {
  return (
    <div className="min-h-screen w-full flex items-center justify-center bg-[#] px-6">
      <div className="container max-w-6xl mx-auto flex flex-col lg:flex-row items-center justify-between gap-12 py-8">
        <div className="w-full lg:w-1/2 order-2 lg:order-1">
          <div className="max-w-lg mx-auto lg:mx-0">
            <h1 className="text-4xl md:text-6xl font-bold text-white mb-4">
              OOOps!
            </h1>
            <h2 className="text-2xl md:text-3xl font-semibold text-gray-200 mb-4">
              Page Not Found
            </h2>
            <p className="text-gray-300 text-base md:text-lg mb-8">
              Sorry about that! Please visit our homepage to get where you need
              to go.
            </p>
            <Link
              href="/"
              className="inline-flex items-center px-6 py-3 text-base font-medium text-[#122638] bg-[#37B7C3] hover:bg-gray-100 rounded-md transition-colors duration-200"
            >
              Back to homepage
            </Link>
          </div>
        </div>

        <div className="w-full lg:w-1/2 order-1 lg:order-2">
          <Image
            src="/404.svg"
            alt="404 Illustration"
            width={476}
            height={490}
            priority
            className="w-full max-w-[476px] mx-auto"
          />
        </div>
      </div>
    </div>
  );
}
