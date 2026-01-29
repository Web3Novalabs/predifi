import Navbar from "../(marketing)/components/NavBar";
import Footer from "../(marketing)/components/Footer";
import WaitlistForm from "./components/WaitlistForm";

export default function WaitlistPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <Navbar />

      <main className="w-screen overflow-x-hidden">
        <WaitlistForm />
        <Footer />
      </main>
    </div>
  );
}
