import type { Metadata } from "next";
import Navbar from "../(marketing)/components/NavBar";

export const metadata: Metadata = { title: "Join the Waitlist" };
import Footer from "../(marketing)/components/Footer";
import WaitlistForm from "./components/WaitlistForm";

export default function WaitlistPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112] flex flex-col">
      <Navbar />

      <main className="w-screen overflow-x-hidden flex-1">
        <WaitlistForm />
      </main>
      
      <Footer />
    </div>
  );
}
