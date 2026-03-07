import Nav from "@/components/Nav";
import Hero from "@/components/Hero";
import Problem from "@/components/Problem";
import LiveFeeds from "@/components/LiveFeeds";
import HowItWorks from "@/components/HowItWorks";
import CostComparison from "@/components/CostComparison";
import ZeroMigration from "@/components/ZeroMigration";
import BeyondBasic from "@/components/BeyondBasic";
import Footer from "@/components/Footer";

export default function Home() {
  return (
    <main className="min-h-screen bg-bg">
      <Nav />
      <Hero />
      <Problem />
      <LiveFeeds />
      <HowItWorks />
      <CostComparison />
      <ZeroMigration />
      <BeyondBasic />
      <Footer />
    </main>
  );
}
