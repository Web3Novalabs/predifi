import Image, { StaticImageData } from "next/image";
import img1 from "@/public/whyChose1.svg";
import img2 from "@/public/whyChose2.svg";
import img3 from "@/public/whyChose3.svg";
import img4 from "@/public/whyChose4.svg";

// Type definition for the card props
interface PredictionCardProps {
    image: StaticImageData;
    title: string;
    description: string;
}

// Reusable Card Component
const PredictionCard: React.FC<PredictionCardProps> = ({
    image,
    title,
    description,
}) => {
    return (
        <div className="border rounded-[8px] flex flex-col lg:flex-row justify-between items-center gap-3 sm:gap-4 w-full font-work lg:pr-3">
            <Image
                className="w-full h-full rounded-[8px] lg:self-start object-cover"
                src={image}
                alt={title}
            />
            <div className="grid gap-4 p-5 lg:p-0">
                <h2 className="capitalize font-semibold xl:text-lg">{title}</h2>
                <p className="font-normal text-xs">{description}</p>
                <button className="w-fit border border-[#373737] px-3 py-[1px] sm:py-1 text-center rounded-full capitalize">
                    Learn more
                </button>
            </div>
        </div>
    );
};

// Configuration for prediction type cards
const PREDICTION_TYPES: PredictionCardProps[] = [
    {
        image: img1,
        title: "Decentralized and Transparent",
        description:
            "The Win Bet is a straightforward prediction pool where participants choose between two clear outcomes.",
    },
    {
        image: img2,
        title: "No Coding Required",
        description:
            "The Win Bet is a straightforward prediction pool where participants choose between two clear outcomes.",
    },
    {
        image: img3,
        title: "Profit While Engaging",
        description:
            "The Win Bet is a straightforward prediction pool where participants choose between two clear outcomes.",
    },
    {
        image: img4,
        title: "Community-Driven Predictions",
        description:
            "The Win Bet is a straightforward prediction pool where participants choose between two clear outcomes.",
    },
];

// Main Component
function PredictionType() {
    return (
        <div className="container mx-auto px-4 sm:px-6 lg:px-8">
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-2 justify-center gap-5">
                {PREDICTION_TYPES.map((type, index) => (
                    <PredictionCard
                        key={index}
                        image={type.image}
                        title={type.title}
                        description={type.description}
                    />
                ))}
            </div>
        </div>
    );
}

export default PredictionType;
