import Nav from "@/components/Nav";
import Hero from "@/components/Hero";
import Problem from "@/components/Problem";
import HowItWorks from "@/components/HowItWorks";
import CostComparison from "@/components/CostComparison";
import ZeroMigration from "@/components/ZeroMigration";
import L2Economics from "@/components/L2Economics";
import BeyondBasic from "@/components/BeyondBasic";
import FAQ from "@/components/FAQ";
import Footer from "@/components/Footer";

export default function Home() {
  return (
    <main className="min-h-screen bg-bg text-text-primary">
      <Nav />
      <Hero />
      <Problem />
      <HowItWorks />
      <CostComparison />
      <ZeroMigration />
      <L2Economics />
      <BeyondBasic />
      <FAQ />
      <Footer />
    </main>
  );
}
