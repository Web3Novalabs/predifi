const TabNavigation = () => (
    <div className="flex gap-[13px]">
      {["Dashboard", "Profile"].map((tab) => (
        <button
          key={tab}
          className="w-[170px] h-[44px] bg-[#373737] hover:text-[whitesmoke] transition-all duration-200 font-['Work_sans'] font-[300] rounded-t-[48px] text-[#CCCCCC]"
        >
          {tab}
        </button>
      ))}
    </div>
  );
  
  export default TabNavigation;
  