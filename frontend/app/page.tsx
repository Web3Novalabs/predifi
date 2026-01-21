import Navbar from "./(marketing)/components/Navbar";
import HeroSection from "./(marketing)/components/HeroSection";
import PredictionProtocol from "./(marketing)/components/PredictionProtocol";
import Features from "./(marketing)/components/Features";
import InstinctsToSignals from "./(marketing)/components/InstinctsToSignals";
import FAQ from "./(marketing)/components/FAQ";
import Footer from "./(marketing)/components/Footer";

export default function Home() {
  return (
    <div className="text-sm min-h-screen bg-[#001112]">
      <main className="w-screen overflow-x-hidden">
        <Navbar />
        <HeroSection />
        <div className="relative space-y-10 lg:space-y-[150px] pt-[80px] lg:pt-[180px]">
          <img
            src="/gradient.png"
            alt=""
            aria-hidden="true"
            className="absolute top-0 left-0 w-full pointer-events-none z-0"
          />
          <PredictionProtocol />
          <Features />
          <InstinctsToSignals />
        </div>
        <FAQ />
        <Footer />
      </main>
    </div>
  );
}
