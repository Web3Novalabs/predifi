import Navbar from "../(marketing)/components/NavBar";
import Footer from "../(marketing)/components/Footer";
import Hero from "./components/Hero";
import Mission from "./components/Mission";
import HowItWorks from "./components/HowItWorks";
import Benefits from "./components/Benefits";

export default function AboutPage() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <Navbar />

      <main className="w-screen overflow-x-hidden">
        <Hero />
        <Mission />
        <HowItWorks />
        <Benefits />
        <Footer />
      </main>
    </div>
  );
}